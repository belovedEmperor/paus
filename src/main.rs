use clap::Parser as _;
use paus::cli::{Cli, handle_cli};
use std::error::Error;

#[tokio::main]
/// Parses CLI args and dispatches to the appropriate handler.
///
/// # Errors
///
/// Returns an error if the dispatched command fails.
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    handle_cli(&cli).await?;

    Ok(())
}
