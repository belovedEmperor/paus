use std::{error::Error, fs::OpenOptions, io::Write as _};

use serde_json::json;

use crate::stopwatch::StopwatchState;

/// Appends a completed phase record to `~/.local/share/paus/history.jsonl`.
///
/// Each line is a JSON object with the RFC 3339 timestamp when the phase ended,
/// the phase kind, and the elapsed duration in seconds. The directory is created
/// if it does not already exist.
///
/// # Errors
///
/// Returns an error if the data directory cannot be resolved, the directory
/// cannot be created, serialization fails, or the file cannot be opened or written.
pub fn append_history(state: &StopwatchState) -> Result<(), Box<dyn Error>> {
    let share_dir = dirs::data_local_dir().ok_or("Failed to find local share dir")?;
    let path = share_dir.join("paus/history.jsonl");

    std::fs::create_dir_all(path.parent().ok_or("Failed to get ~/.local/share/paus")?)?;

    let entry = json!({
        "ended_at": chrono::Local::now().to_rfc3339(),
        "phase": state.phase,
        "seconds": state.get_elapsed_seconds()
    });

    let mut bytes = serde_json::to_vec(&entry)?;
    bytes.push(b'\n');

    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    file.write_all(&bytes)?;

    Ok(())
}
