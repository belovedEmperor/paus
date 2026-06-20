use serde_json::json;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _},
    net::UnixStream,
};

use crate::{
    server::run_daemon,
    stopwatch::{Phase, StopwatchStatus},
};
use std::error::Error;

#[derive(clap::Parser)]
#[command(name = "paus")]
#[command(version = "0.1.0")]
#[command(about = "A Third Time stopwatch with daemon support", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    Status {
        #[arg(long)]
        focus: bool,
        #[arg(long)]
        breaks: bool,
        #[arg(long)]
        balance: bool,
    },
    Focus,
    Break,
    TogglePhase,
    Pause,
    Unpause,
    TogglePause,
}

#[derive(clap::Subcommand)]
pub enum DaemonAction {
    Run,
    Stop,
}

/// Dispatches the parsed CLI command to the appropriate handler.
///
/// # Errors
///
/// Returns an error if any command fails (daemon I/O, socket communication, or JSON parsing).
pub async fn handle_cli(cli: &Cli) -> Result<(), Box<dyn Error>> {
    match &cli.command {
        Some(Commands::Daemon { action }) => handle_daemon(action).await?,
        Some(Commands::Status {
            focus,
            breaks,
            balance,
        }) => {
            let raw = send_command("status").await?;
            let value: serde_json::Value = serde_json::from_str(&raw)?;
            let stopwatch_status: StopwatchStatus =
                serde_json::from_value(value.get("data").ok_or("no data")?.clone())?;

            let stopwatch_status = stopwatch_status.to_minutes();
            let icon = if stopwatch_status.is_paused {
                "⏸"
            } else {
                "▶"
            };

            let mut parts = vec![];

            let dynamic = !focus && !breaks && !balance;

            if *focus
                || (dynamic
                    && (stopwatch_status.phase == Phase::Idle
                        || stopwatch_status.phase == Phase::Focusing))
            {
                parts.push(format!(
                    "⏰ {:02}:{:02}",
                    stopwatch_status.focused_seconds / 60,
                    stopwatch_status.focused_seconds % 60
                ));
            }
            if *breaks || (dynamic && stopwatch_status.phase == Phase::Breaking) {
                parts.push(format!(
                    "🏖️ {:02}:{:02}",
                    stopwatch_status.breaked_seconds / 60,
                    stopwatch_status.breaked_seconds % 60
                ));
            }
            if *balance || dynamic {
                parts.push(format!(
                    "⚖️ {:02}:{:02}",
                    stopwatch_status.balance / 60,
                    stopwatch_status.balance % 60
                ));
            }

            parts.push(icon.to_owned());

            println!("{}", parts.join(" "));
        }
        Some(Commands::Focus) => {
            let response = send_command("focus").await?;
            print!("{response}");
        }
        Some(Commands::Break) => {
            let response = send_command("break").await?;
            print!("{response}");
        }
        Some(Commands::TogglePhase) => {
            let response = send_command("toggle-phase").await?;
            print!("{response}");
        }
        Some(Commands::Pause) => {
            let response = send_command("pause").await?;
            print!("{response}");
        }
        Some(Commands::Unpause) => {
            let response = send_command("unpause").await?;
            print!("{response}");
        }
        Some(Commands::TogglePause) => {
            let response = send_command("toggle-pause").await?;
            print!("{response}");
        }
        None => {}
    }

    Ok(())
}

/// Handles `daemon` subcommands: starts the daemon or sends a stop request.
///
/// # Errors
///
/// Returns an error if starting the daemon fails or the stop command cannot be sent.
pub async fn handle_daemon(action: &DaemonAction) -> Result<(), Box<dyn Error>> {
    match action {
        DaemonAction::Run => run_daemon().await?,
        DaemonAction::Stop => {
            let response = send_command("daemon-stop").await?;
            print!("{response}");
        }
    }

    Ok(())
}

/// Sends a JSON command to the running daemon over a Unix socket and returns the raw response line.
///
/// # Errors
///
/// Returns an error if the runtime directory is unavailable, the socket connection fails,
/// or reading/writing to the stream fails.
async fn send_command(command: &str) -> Result<String, Box<dyn Error>> {
    let runtime_dir = dirs::runtime_dir().ok_or("no runtime dir")?;
    let mut stream = UnixStream::connect(runtime_dir.join("paus.sock")).await?;

    let mut request = serde_json::to_string(&json!({ "command": command }))?;
    request.push('\n');
    stream.write_all(request.as_bytes()).await?;

    let mut reader = tokio::io::BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    Ok(response)
}
