use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::UnixStream,
};

use crate::server::run_daemon;
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
    Status,
    Focus,
    Break,
    Pause,
    Unpause,
}

#[derive(clap::Subcommand)]
pub enum DaemonAction {
    Run,
}

pub async fn handle_cli(cli: &Cli) -> Result<(), Box<dyn Error>> {
    match &cli.command {
        Some(Commands::Daemon { action }) => handle_daemon(action).await?,
        Some(Commands::Status) => {
            let response = send_command("status").await?;
            print!("{response}");
        }
        Some(Commands::Focus) => {
            let response = send_command("focus").await?;
            print!("{response}");
        }
        Some(Commands::Break) => {
            let response = send_command("break").await?;
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
        None => {}
    }

    Ok(())
}

pub async fn handle_daemon(action: &DaemonAction) -> Result<(), Box<dyn Error>> {
    match action {
        DaemonAction::Run => run_daemon().await?,
    }

    Ok(())
}

async fn send_command(command: &str) -> Result<String, Box<dyn Error>> {
    let runtime_dir = dirs::runtime_dir().ok_or("no runtime dir")?;
    let mut stream = UnixStream::connect(runtime_dir.join("paus.sock")).await?;

    let request = format!("{{\"command\":\"{command}\"}}\n");
    stream.write_all(request.as_bytes()).await?;

    let mut reader = tokio::io::BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    Ok(response)
}
