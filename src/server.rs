use std::error::Error;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _},
    net::{UnixListener, UnixStream},
    signal::unix::{SignalKind, signal},
};

use crate::stopwatch::{Phase, StopwatchState};

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

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => match handle_connection(stream, &mut state).await {
                        Ok(ConnectionOkResult::Stop) => {
                            state.try_save_state()?;
                            break;
                        }
                        Ok(ConnectionOkResult::Ok) => {}
                        Err(error) => {
                            eprintln!("connection error: {error}");
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to accept listener: {error}");
                    }
                }
            }
            _ = sigterm.recv() => {
                state.try_save_state()?;
                break;
            }
            _ = sigint.recv() => {
                state.try_save_state()?;
                break;
            }
            _ = sighup.recv() => {
                state.try_save_state()?;
                break;
            }
        }
    }

    Ok(())
}

#[derive(serde::Deserialize)]
struct Request {
    command: String,
}

#[derive(serde::Serialize)]
struct Response {
    ok: bool,
    message: String,
}

enum ConnectionOkResult {
    Ok,
    Stop,
}

async fn handle_connection(
    stream: UnixStream,
    state: &mut StopwatchState,
) -> Result<ConnectionOkResult, Box<dyn Error>> {
    let (reader, mut writer) = stream.into_split();

    let mut buffer_reader = tokio::io::BufReader::new(reader);

    let mut line = String::new();
    buffer_reader.read_line(&mut line).await?;

    let request: Request = serde_json::from_str(&line)?;

    let response = match request.command.as_str() {
        "daemon-stop" => {
            let mut json = serde_json::to_string(&Response {
                ok: true,
                message: "stopping".to_owned(),
            })?;
            json.push('\n');

            writer.write_all(json.as_bytes()).await?;

            return Ok(ConnectionOkResult::Stop);
        }
        "status" => {
            state.update_times();
            let stopwatch_status = state.get_stopwatch_status().to_minutes();

            Response {
                ok: true,
                message: format!(
                    "focus {}, breaks {}, balance {} {}",
                    stopwatch_status.focused_seconds,
                    stopwatch_status.breaked_seconds,
                    stopwatch_status.balance,
                    if stopwatch_status.is_paused {
                        "■"
                    } else {
                        "▶"
                    }
                ),
            }
        }
        "focus" => {
            state.start_focus();

            Response {
                ok: true,
                message: "started focusing".to_owned(),
            }
        }
        "break" => {
            state.start_break();

            Response {
                ok: true,
                message: "started breaking".to_owned(),
            }
        }
        "toggle-phase" => {
            state.toggle_phase();

            Response {
                ok: true,
                message: match state.phase {
                    Phase::Focusing => "started focusing",
                    Phase::Breaking => "started breaking",
                    Phase::Idle => "started idle",
                }
                .to_owned(),
            }
        }
        "pause" => {
            state.pause();

            Response {
                ok: true,
                message: "paused".to_owned(),
            }
        }
        "unpause" => {
            state.unpause();

            Response {
                ok: true,
                message: "unpaused".to_owned(),
            }
        }
        "toggle-pause" => {
            state.toggle_pause();

            Response {
                ok: true,
                message: if state.is_paused {
                    "paused"
                } else {
                    "unpaused"
                }
                .to_owned(),
            }
        }
        unknown => Response {
            ok: false,
            message: format!("unknown command: {unknown}"),
        },
    };

    let mut json = serde_json::to_string(&response)?;
    json.push('\n');

    writer.write_all(json.as_bytes()).await?;

    Ok(ConnectionOkResult::Ok)
}
