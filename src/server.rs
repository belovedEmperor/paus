use anyhow::{Result, anyhow};
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _},
    net::{UnixListener, UnixStream},
    signal::unix::{SignalKind, signal},
};

use crate::{
    Request, Response,
    cli::{Commands, DaemonAction},
    config::Config,
    history::HistoryEntry,
    stopwatch::{Phase, StopwatchState, now_seconds},
};

/// Starts the daemon: binds a Unix socket, loads or initializes state,
/// and handles incoming connections until a stop command or signal is received.
///
/// # Errors
///
/// Returns an error if the socket cannot be bound, signals cannot be registered,
/// or state persistence fails.
pub async fn run_daemon() -> Result<()> {
    let config = Config::load();
    Config::create_config_file_if_not_existing(&config)?;

    let today = chrono::Local::now().date_naive().to_string();

    let mut state = match StopwatchState::try_read_state(&config.data_dir) {
        Ok(state) if state.last_started_date != today => {
            StopwatchState::new(config.break_ratio, config.data_dir.clone())
        }
        Ok(mut state) => {
            state.break_ratio = config.break_ratio.clone();
            state.phase = Phase::Idle;
            state.is_paused = true;
            state.phase_started_at_seconds = now_seconds();
            state.data_dir = config.data_dir.clone();

            state
        }
        Err(_) => StopwatchState::new(config.break_ratio.clone(), config.data_dir.clone()),
    };

    let runtime_dir = dirs::runtime_dir().ok_or_else(|| anyhow!("Failed to find runtime dir"))?;

    let socket_path = runtime_dir.join("paus.sock");
    if socket_path.try_exists()? {
        std::fs::remove_file(&socket_path)?;
    }
    let listener = UnixListener::bind(socket_path)?;

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sighup = signal(SignalKind::hangup())?;

    let any_signal = async {
        tokio::select! {
            _ = sigterm.recv() => {},
            _ = sigint.recv() => {},
            _ = sighup.recv() => {},
        }
    };
    tokio::pin!(any_signal);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => match handle_connection(stream, &mut state).await {
                        Ok(true) => {
                            state.update_times_and_append_history();
                            state.try_save_state(&config.data_dir)?;
                            break;
                        }
                        Ok(false) => {}
                        Err(error) => {
                            eprintln!("connection error: {error}");
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to accept listener: {error}");
                    }
                }
            }
            () = &mut any_signal => {
                state.update_times_and_append_history();
                state.try_save_state(&config.data_dir)?;
                break;
            }
        }
    }

    Ok(())
}

/// Reads a single JSON command from the stream, updates stopwatch state, and writes back a JSON response.
///
/// Returns `true` when the daemon should shut down after this connection.
///
/// # Errors
///
/// Returns an error if reading/writing the stream fails or JSON (de)serialization fails.
async fn handle_connection(stream: UnixStream, state: &mut StopwatchState) -> Result<bool> {
    let (reader, mut writer) = stream.into_split();

    let mut buffer_reader = tokio::io::BufReader::new(reader);

    let mut line = String::new();
    buffer_reader.read_line(&mut line).await?;

    let request: Request = serde_json::from_str(&line)?;

    let response = match request.command {
        Commands::Daemon {
            action: DaemonAction::Run,
        } => return Err(anyhow!("daemon run is client only")),
        Commands::Daemon {
            action: DaemonAction::Stop,
        } => {
            let mut json = serde_json::to_string(&Response {
                ok: true,
                data: serde_json::to_value("stopping")?,
            })?;
            json.push('\n');

            writer.write_all(json.as_bytes()).await?;

            return Ok(true);
        }
        Commands::Status { .. } => {
            let stopwatch_status = state.get_stopwatch_status();

            Response {
                ok: true,
                data: serde_json::to_value(stopwatch_status)?,
            }
        }
        Commands::Focus => {
            state.start_focus();

            Response {
                ok: true,
                data: serde_json::to_value("started focusing")?,
            }
        }
        Commands::Break => {
            state.start_break();

            Response {
                ok: true,
                data: serde_json::to_value("started breaking")?,
            }
        }
        Commands::TogglePhase => {
            state.toggle_phase();

            Response {
                ok: true,
                data: serde_json::to_value(match state.phase {
                    Phase::Focusing => "started focusing",
                    Phase::Breaking => "started breaking",
                    Phase::Idle => "started idle",
                })?,
            }
        }
        Commands::Pause => {
            state.pause();

            Response {
                ok: true,
                data: serde_json::to_value("paused")?,
            }
        }
        Commands::Unpause => {
            state.unpause();

            Response {
                ok: true,
                data: serde_json::to_value("unpaused")?,
            }
        }
        Commands::TogglePause => {
            state.toggle_pause();

            Response {
                ok: true,
                data: serde_json::to_value(if state.is_paused {
                    "paused"
                } else {
                    "unpaused"
                })?,
            }
        }
        Commands::Add { duration, phase } => {
            let duration_seconds = duration * 60;

            HistoryEntry::append_history(&state.data_dir, phase, duration_seconds)?;
            state.add_duration(phase, duration_seconds);

            Response {
                ok: true,
                data: serde_json::to_value("added history entry")?,
            }
        }
        Commands::Compute => {
            let history = HistoryEntry::read_history(state)?;
            let (focused, breaked) = HistoryEntry::compute_state_durations_from_history(&history);
            state.total_focused_seconds = focused;
            state.total_breaked_seconds = breaked;

            Response {
                ok: true,
                data: serde_json::to_value("computed new state durations")?,
            }
        }
    };

    let mut json = serde_json::to_string(&response)?;
    json.push('\n');

    writer.write_all(json.as_bytes()).await?;

    Ok(false)
}
