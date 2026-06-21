use serde::{Deserialize, Serialize};

pub mod cli;
pub mod server;
pub mod stopwatch;

#[derive(Deserialize)]
struct Request {
    command: String,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    ok: bool,
    data: serde_json::Value,
}
