use std::error::Error;
use tokio::net::{UnixListener, UnixStream};

pub enum Phase {
    Idle,
    Focusing,
    Breaking,
}

#[derive(Clone)]
pub enum BreakRatio {
    Lazy = 2,
    Standard = 3,
    Industrious = 4,
    Hard = 5,
    Grinding = 6,
}

pub struct State {
    pub phase: Phase,
    pub phase_started_at_seconds: u64,
    pub total_focused_seconds: u64,
    pub total_breaked_seconds: u64,
    pub balance: i128,
    pub break_ratio: BreakRatio,
}

pub async fn run_daemon() -> Result<(), Box<dyn Error>> {
    let mut state = State {
        phase: Phase::Idle,
        phase_started_at_seconds: 0,
        total_focused_seconds: 300,
        total_breaked_seconds: 120,
        balance: 0,
        break_ratio: BreakRatio::Standard,
    };

    let runtime_dir = dirs::runtime_dir().ok_or_else(|| "Failed to find runtime dir")?;
    let listener = UnixListener::bind(runtime_dir.join("paus.sock"))?;

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                println!("YES!");
            }
            Err(error) => {
                eprintln!("NOPE?! {error}")
            }
        }
    }

    Ok(())
}

async fn handle_connection(stream: UnixStream, state: &mut State) -> Result<(), Box<dyn Error>> {
    Ok(())
}
