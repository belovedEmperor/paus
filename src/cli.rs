use crate::server::{State, run_daemon};
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
}

#[derive(clap::Subcommand)]
pub enum DaemonAction {
    Run,
}

pub async fn handle_cli(cli: &Cli) -> Result<(), Box<dyn Error>> {
    match &cli.command {
        Some(Commands::Daemon { action }) => handle_daemon(action).await?,
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

pub fn calculate_balance(state: &mut State) -> i128 {
    let balance = (Into::<i128>::into(state.total_focused_seconds)
        / state.break_ratio.clone() as i128)
        - Into::<i128>::into(state.total_breaked_seconds);

    set_balance(state, balance);

    balance
}

pub fn set_balance(state: &mut State, balance: i128) {
    state.balance = balance;
}

pub fn status(state: &State, balance_seconds: i128) {
    let _ = format!(
        "focus_emoji {} | break_emoji {} | balance_emoji {}",
        to_minutes(state.total_focused_seconds.into()),
        to_minutes(state.total_breaked_seconds.into()),
        to_minutes(balance_seconds)
    );
}

fn to_minutes(seconds: i128) -> i128 {
    seconds / 60
}
