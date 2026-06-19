use clap::Parser as _;
use paus::cli::{Cli, handle_cli};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    handle_cli(&cli).await?;

    Ok(())
}
