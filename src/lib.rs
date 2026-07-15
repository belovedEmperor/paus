use serde::{Deserialize, Serialize};

use crate::cli::Commands;

pub mod cli;
pub mod config;
pub mod history;
pub mod server;
pub mod stopwatch;

#[derive(Serialize, Deserialize)]
struct Request {
    command: Commands,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    ok: bool,
    data: serde_json::Value,
}
