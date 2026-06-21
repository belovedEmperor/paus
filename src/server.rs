use std::error::Error;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _},
    net::{UnixListener, UnixStream},
    signal::unix::{SignalKind, signal},
};

use crate::{
    Request, Response,
    stopwatch::{Phase, StopwatchState},
};

/// Starts the daemon: binds a Unix socket, loads or initializes state,
/// and handles incoming connections until a stop command or signal is received.
///
/// # Errors
///
/// Returns an error if the socket cannot be bound, signals cannot be registered,
/// or state persistence fails.
pub async fn run_daemon() -> Result<(), Box<dyn Error>> {
    let mut state = match StopwatchState::try_read_state() {
        Ok(state) => state,
        Err(_) => StopwatchState::new(crate::stopwatch::BreakRatio::Standard),
    };

    let runtime_dir = dirs::runtime_dir().ok_or("Failed to find runtime dir")?;

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
                            state.try_save_state()?;
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
                state.try_save_state()?;
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
async fn handle_connection(
    stream: UnixStream,
    state: &mut StopwatchState,
) -> Result<bool, Box<dyn Error>> {
    let (reader, mut writer) = stream.into_split();

    let mut buffer_reader = tokio::io::BufReader::new(reader);

    let mut line = String::new();
    buffer_reader.read_line(&mut line).await?;

    let request: Request = serde_json::from_str(&line)?;

    let response = match request.command.as_str() {
        "daemon-stop" => {
            let mut json = serde_json::to_string(&Response {
                ok: true,
                data: serde_json::to_value("stopping")?,
            })?;
            json.push('\n');

            writer.write_all(json.as_bytes()).await?;

            return Ok(true);
        }
        "status" => {
            state.update_times();
            let stopwatch_status = state.get_stopwatch_status();

            Response {
                ok: true,
                data: serde_json::to_value(stopwatch_status)?,
            }
        }
        "focus" => {
            state.start_focus();

            Response {
                ok: true,
                data: serde_json::to_value("started focusing")?,
            }
        }
        "break" => {
            state.start_break();

            Response {
                ok: true,
                data: serde_json::to_value("started breaking")?,
            }
        }
        "toggle-phase" => {
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
        "pause" => {
            state.pause();

            Response {
                ok: true,
                data: serde_json::to_value("paused")?,
            }
        }
        "unpause" => {
            state.unpause();

            Response {
                ok: true,
                data: serde_json::to_value("unpaused")?,
            }
        }
        "toggle-pause" => {
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
        unknown => Response {
            ok: false,
            data: serde_json::to_value(format!("unknown command: {unknown}"))?,
        },
    };

    let mut json = serde_json::to_string(&response)?;
    json.push('\n');

    writer.write_all(json.as_bytes()).await?;

    Ok(false)
}
