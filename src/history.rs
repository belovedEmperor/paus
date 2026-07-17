use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{BufRead as _, BufReader, Write as _},
    path::Path,
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
    /// Appends a completed phase record to `data_dir` + `/history.jsonl`.
    ///
    /// Each line is a JSON object with the RFC 3339 timestamp when the phase ended,
    /// the phase kind, and the elapsed duration in seconds (passed by the caller,
    /// captured before the phase timer is reset). The directory is created if it does
    /// not already exist.
    ///
    /// # Errors
    //
    /// Returns an error if the data directory cannot be resolved, the directory
    /// cannot be created, serialization fails, or the file cannot be opened or written.
    pub fn append_history(
        data_dir: &Path,
        phase: Phase,
        seconds: u64,
    ) -> Result<(), Box<dyn Error>> {
        let path = data_dir.join("history.jsonl");

        std::fs::create_dir_all(path.parent().ok_or("Failed to get data_dir")?)?;

        let entry = Self {
            ended_at: chrono::Local::now().to_rfc3339(),
            phase,
            seconds,
        };

        let mut bytes = serde_json::to_vec(&entry)?;
        bytes.push(b'\n');

        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        file.write_all(&bytes)?;

        Ok(())
    }

    /// Reads all entries from `data_dir` + `/history.jsonl`.
    ///
    /// Returns an empty vector if the file does not exist yet. Malformed lines
    /// are silently skipped.
    ///
    /// # Errors
    ///
    /// Returns an error if the data directory cannot be resolved, the directory
    /// cannot be created, or the file cannot be read.
    pub fn read_history(state: &StopwatchState) -> Result<Vec<Self>, Box<dyn Error>> {
        let path = state.data_dir.join("history.jsonl");

        std::fs::create_dir_all(path.parent().ok_or("Failed to get data_dir")?)?;

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

    /// Computes the state durations from today's history entries
    pub fn compute_state_durations_from_history(history: &[Self]) -> (u64, u64) {
        history
            .iter()
            .filter(|entry| {
                entry.ended_at.split('T').next()
                    == chrono::Local::now().to_rfc3339().split('T').next()
            })
            .fold((0, 0), |(focused, breaked), entry| match entry.phase {
                Phase::Focusing => (focused + entry.seconds, breaked),
                Phase::Breaking => (focused, breaked + entry.seconds),
                Phase::Idle => (focused, breaked),
            })
    }
}
