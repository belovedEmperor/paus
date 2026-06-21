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
#[command(
    about = "A Third Time stopwatch with daemon support",
    long_about = "A stopwatch based on the Third Time productivity method.

Tracks focused time and break time, maintaining a balance.
Focusing adds to the balance at a ratio.
Breaking withdraws from the balance.

Runs a background daemon that persists between commands and restarts."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    #[command(about = "Manage stopwatch daemon")]
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    #[command(
        about = "Get stopwatch status",
        long_about = "Show current stopwatch status
By default shows current phase, balance, and pause state dynamically"
    )]
    Status {
        #[arg(long, help = "Show focus time")]
        focus: bool,
        #[arg(long, help = "Show break time")]
        breaks: bool,
        #[arg(long, help = "Show balance")]
        balance: bool,
    },
    #[command(about = "Start focusing")]
    Focus,
    #[command(about = "Start breaking")]
    Break,
    #[command(about = "Toggle focus/break")]
    TogglePhase,
    #[command(about = "Pause stopwatch")]
    Pause,
    #[command(about = "Unpause stopwatch")]
    Unpause,
    #[command(about = "Toggle stopwatch pause")]
    TogglePause,
}

#[derive(clap::Subcommand)]
pub enum DaemonAction {
    #[command(about = "Run daemon")]
    Run,
    #[command(about = "Stop daemon")]
    Stop,
}

/// Dispatches the parsed CLI command to the appropriate handler.
///
/// # Errors
///
/// Returns an error if any command fails (daemon I/O, socket communication, or JSON parsing).
pub async fn handle_cli(cli: &Cli) -> Result<(), Box<dyn Error>> {
    match &cli.command {
        Some(Commands::Daemon { action }) => match action {
            DaemonAction::Run => run_daemon().await?,
            DaemonAction::Stop => {
                let response = send_command("daemon-stop").await?;
                print!("{response}");
            }
        },
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
                let mut negative = false;

                let balance_minutes = stopwatch_status.balance % 60;
                if balance_minutes < 0 {
                    negative = true;
                }

                parts.push(format!(
                    "⚖️ {}{:02}:{:02}",
                    { if negative { "-" } else { "" } },
                    stopwatch_status.balance / 60,
                    balance_minutes.abs()
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
