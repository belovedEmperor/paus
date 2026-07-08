use serde::{Deserialize, Serialize};

pub mod cli;
pub mod config;
pub mod history;
pub mod server;
pub mod stopwatch;

#[derive(Deserialize)]
struct Request {
    command: String,
    #[serde(default)]
    data: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    ok: bool,
    data: serde_json::Value,
}
