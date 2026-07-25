use serde::{Deserialize, Serialize};

use crate::cli::Commands;

pub mod cli;
pub mod config;
pub mod history;
pub mod server;
pub mod stopwatch;
pub mod tui;

#[derive(Serialize, Deserialize)]
struct Request {
    command: Commands,
}

/// JSON reply written back to the CLI client over the daemon socket.
#[derive(Serialize, Deserialize)]
pub struct Response {
    ok: bool,
    data: serde_json::Value,
}
