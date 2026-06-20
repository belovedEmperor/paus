use std::error::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};

use crate::stopwatch::{BreakRatio, Phase, StopwatchState};

pub async fn run_daemon() -> Result<(), Box<dyn Error>> {
    let mut state = StopwatchState::new(0, 0, BreakRatio::Standard);

    let runtime_dir = dirs::runtime_dir().ok_or_else(|| "Failed to find runtime dir")?;
    let listener = UnixListener::bind(runtime_dir.join("paus.sock"))?;

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                if let Err(error) = handle_connection(stream, &mut state).await {
                    eprintln!("connection error: {error}");
                }
            }
            Err(error) => {
                eprintln!("NOPE?! {error}")
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

async fn handle_connection(
    stream: UnixStream,
    state: &mut StopwatchState,
) -> Result<(), Box<dyn Error>> {
    let (reader, mut writer) = stream.into_split();

    let mut buffer_reader = tokio::io::BufReader::new(reader);

    let mut line = String::new();
    buffer_reader.read_line(&mut line).await?;

    let request: Request = serde_json::from_str(&line)?;

    let response = match request.command.as_str() {
        "status" => {
            state.update_times();
            let stopwatch_status = state.get_stopwatch_status();

            Response {
                ok: true,
                message: format!(
                    "focus {}, breaks {}, balance {}",
                    stopwatch_status.focused_seconds,
                    stopwatch_status.breaked_seconds,
                    stopwatch_status.balance
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

    Ok(())
}
