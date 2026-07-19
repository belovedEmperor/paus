use serde::{Deserialize, Serialize};
use serde_json::{from_str, json};
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _},
    net::UnixStream,
};

use crate::{
    Request, Response,
    server::run_daemon,
    stopwatch::{Phase, StopwatchStatus},
};
use std::error::Error;

#[derive(clap::Parser)]
#[command(name = "paus")]
#[command(version = "0.3.0")]
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

#[derive(clap::Subcommand, Serialize, Deserialize)]
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
    #[command(about = "Manually add entry to history & time to state")]
    Add {
        #[arg(short, long, help = "Entry duration in minutes")]
        duration: u64,
        #[arg(short, long, help = "Entry phase, focusing or breaking")]
        phase: Phase,
    },
    #[command(about = "Compute new state durations from history entries")]
    Compute,
}

#[derive(clap::Subcommand, Serialize, Deserialize)]
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
                let response = send_command(Commands::Daemon {
                    action: DaemonAction::Stop,
                })
                .await?;
                let response = format_response(response.as_str())?;
                print!("{response}");
            }
        },
        Some(Commands::Add { duration, phase }) => {
            let response = send_command(Commands::Add {
                duration: *duration,
                phase: *phase,
            })
            .await?;
            let response = format_response(response.as_str())?;
            print!("{response}");
        }
        Some(Commands::Status {
            focus,
            breaks,
            balance,
        }) => {
            let raw = send_command(Commands::Status {
                focus: false,
                breaks: false,
                balance: false,
            })
            .await?;
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
            if dynamic && stopwatch_status.phase == Phase::Idle {
                parts.push(format!(
                    "✋ {:02}:{:02}",
                    stopwatch_status.focused_duration / 60,
                    stopwatch_status.focused_duration % 60
                ));
            }
            if *focus || (dynamic && stopwatch_status.phase == Phase::Focusing) {
                parts.push(format!(
                    "⏰ {:02}:{:02}",
                    stopwatch_status.focused_duration / 60,
                    stopwatch_status.focused_duration % 60
                ));
            }
            if *breaks || (dynamic && stopwatch_status.phase == Phase::Breaking) {
                parts.push(format!(
                    "🏖️ {:02}:{:02}",
                    stopwatch_status.breaked_duration / 60,
                    stopwatch_status.breaked_duration % 60
                ));
            }
            if *balance || dynamic {
                let mut negative = false;

                let balance_minutes = stopwatch_status.balance % 60;
                let balance_hours = stopwatch_status.balance / 60;

                if balance_minutes < 0 || balance_hours < 0 {
                    negative = true;
                }

                parts.push(format!(
                    "⚖️ {}{:02}:{:02}",
                    { if negative { "-" } else { "" } },
                    balance_hours.abs(),
                    balance_minutes.abs()
                ));
            }

            parts.push(icon.to_owned());

            println!("{}", parts.join(" "));
        }
        Some(Commands::Focus) => {
            let response = send_command(Commands::Focus).await?;
            let response = format_response(response.as_str())?;
            print!("{response}");
        }
        Some(Commands::Break) => {
            let response = send_command(Commands::Break).await?;
            let response = format_response(response.as_str())?;
            print!("{response}");
        }
        Some(Commands::TogglePhase) => {
            let response = send_command(Commands::TogglePhase).await?;
            let response = format_response(response.as_str())?;
            print!("{response}");
        }
        Some(Commands::Pause) => {
            let response = send_command(Commands::Pause).await?;
            let response = format_response(response.as_str())?;
            print!("{response}");
        }
        Some(Commands::Unpause) => {
            let response = send_command(Commands::Unpause).await?;
            let response = format_response(response.as_str())?;
            print!("{response}");
        }
        Some(Commands::TogglePause) => {
            let response = send_command(Commands::TogglePause).await?;
            let response = format_response(response.as_str())?;
            print!("{response}");
        }
        Some(Commands::Compute) => {
            let response = send_command(Commands::Compute).await?;
            let response = format_response(response.as_str())?;
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
pub async fn send_command(command: Commands) -> Result<String, Box<dyn Error>> {
    let runtime_dir = dirs::runtime_dir().ok_or("no runtime dir")?;
    let mut stream = UnixStream::connect(runtime_dir.join("paus.sock")).await?;

    let mut request = serde_json::to_string(&json!(&Request { command }))?;
    request.push('\n');
    stream.write_all(request.as_bytes()).await?;

    let mut reader = tokio::io::BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    Ok(response)
}

/// Format server response from json to string.
///
/// # Errors
///
/// Returns an error if the response fails to serialize into a `Response`.
fn format_response(response: &str) -> Result<String, Box<dyn Error>> {
    let formatted_response: Response = from_str(response)?;
    Ok(formatted_response
        .data
        .as_str()
        .map_or_else(|| formatted_response.data.to_string(), str::to_owned))
}
