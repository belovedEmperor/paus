use std::{path::PathBuf, thread::sleep, time::Duration};

use clap::Parser as _;

#[derive(clap::Parser)]
#[command(name = "paus")]
#[command(version = "0.1.0")]
#[command(about = "A Third Time stopwatch with daemon support", long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    Wait {
        #[arg(short, long)]
        seconds: u32,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Wait { seconds }) => wait(seconds),
        None => {}
    }

    println!("Hello, world!");
}

fn wait(seconds: u32) {
    let early = chrono::Utc::now();

    sleep(Duration::from_secs(seconds.into()));

    let late = chrono::Utc::now();

    let delta = late - early;

    println!("The calculated time difference is: {}", delta.num_seconds());
}

enum Phase {
    Idle,
    Focusing,
    Breaking,
}

enum BreakRatio {
    Lazy,
    Standard,
    Industrious,
    Hard,
    Grinding,
}

struct State {
    phase: Phase,
    phase_started_at_seconds: u64,
    total_focused_seconds: u64,
    total_breaked_seconds: u64,
    balance: i128,
    alarm_started_at_seconds: u64,
    break_ratio: BreakRatio,
}
