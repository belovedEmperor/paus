use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{BufRead as _, BufReader, Write as _},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::stopwatch::{Phase, StopwatchState};

#[derive(Serialize, Deserialize)]
pub struct HistoryEntry {
    pub ended_at: String,
    pub phase: Phase,
    pub seconds: u64,
}

impl HistoryEntry {
    /// Returns the path to `~/.local/share/paus/history.jsonl`.
    ///
    /// # Errors
    ///
    /// Returns an error if the local data directory cannot be resolved.
    pub fn get_history_path() -> Result<PathBuf, Box<dyn Error>> {
        let share_dir = dirs::data_local_dir().ok_or("Failed to find local share dir")?;
        let path = share_dir.join("paus/history.jsonl");

        Ok(path)
    }

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
    pub fn append_history(state: &StopwatchState, seconds: u64) -> Result<(), Box<dyn Error>> {
        let path = Self::get_history_path()?;

        std::fs::create_dir_all(path.parent().ok_or("Failed to get ~/.local/share/paus")?)?;

        let entry = Self {
            ended_at: chrono::Local::now().to_rfc3339(),
            phase: state.phase.clone(),
            seconds,
        };

        let mut bytes = serde_json::to_vec(&entry)?;
        bytes.push(b'\n');

        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        file.write_all(&bytes)?;

        Ok(())
    }

    /// Reads all entries from `~/.local/share/paus/history.jsonl`.
    ///
    /// Returns an empty vector if the file does not exist yet. Malformed lines
    /// are silently skipped.
    ///
    /// # Errors
    ///
    /// Returns an error if the data directory cannot be resolved, the directory
    /// cannot be created, or the file cannot be read.
    pub fn read_history() -> Result<Vec<Self>, Box<dyn Error>> {
        let path = Self::get_history_path()?;

        std::fs::create_dir_all(path.parent().ok_or("Failed to get ~/.local/share/paus")?)?;

        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(error) => return Err(error.into()),
        };

        let entries: Vec<Self> = BufReader::new(file)
            .lines()
            .filter_map(|line| {
                let line = line.ok()?;
                serde_json::from_str(&line).ok()
            })
            .collect();

        Ok(entries)
    }
}
