use std::{
    error::Error,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum Phase {
    Idle,
    Focusing,
    Breaking,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum BreakRatio {
    Lazy = 2,
    Standard = 3,
    Industrious = 4,
    Hard = 5,
    Grinding = 6,
}

#[derive(Serialize, Deserialize)]
pub struct StopwatchState {
    pub is_paused: bool,
    pub phase: Phase,
    pub phase_started_at_seconds: u64,
    pub total_focused_seconds: u64,
    pub total_breaked_seconds: u64,
    pub break_ratio: BreakRatio,
}

#[derive(Serialize)]
pub struct StopwatchStatus {
    pub is_paused: bool,
    pub phase: Phase,
    pub focused_seconds: u64,
    pub breaked_seconds: u64,
    pub balance: i128,
}

pub fn calculate_balance(state: &StopwatchState) -> i128 {
    (Into::<i128>::into(state.total_focused_seconds) / state.break_ratio.clone() as i128)
        - Into::<i128>::into(state.total_breaked_seconds)
}

impl StopwatchState {
    pub fn new(break_ratio: BreakRatio) -> Self {
        Self {
            is_paused: true,
            phase: Phase::Idle,
            phase_started_at_seconds: now_seconds(),
            total_focused_seconds: 0,
            total_breaked_seconds: 0,
            break_ratio,
        }
    }

    pub fn try_read_state() -> Result<StopwatchState, Box<dyn Error>> {
        let share_dir = dirs::data_local_dir().ok_or("Failed to find local share dir")?;

        let bytes = std::fs::read(share_dir.join("paus/state.json"))?;

        let mut state: StopwatchState = serde_json::from_slice(&bytes)?;

        state.is_paused = true;
        state.phase_started_at_seconds = now_seconds();

        Ok(state)
    }

    pub fn try_save_state(&self) -> Result<(), Box<dyn Error>> {
        let share_dir = dirs::data_local_dir().ok_or("Failed to find local share dir")?;
        let path = share_dir.join("paus/state.json");

        std::fs::create_dir_all(path.parent().ok_or("Failed to get ~/.local/share/paus")?)?;

        let bytes = serde_json::to_vec(self)?;

        std::fs::write(path, bytes)?;

        Ok(())
    }

    pub fn update_times(&mut self) {
        let elapsed_seconds = self.get_elapsed_seconds();

        match self.phase {
            Phase::Focusing => {
                self.total_focused_seconds += elapsed_seconds;
            }
            Phase::Breaking => {
                self.total_breaked_seconds += elapsed_seconds;
            }
            Phase::Idle => {}
        }

        self.phase_started_at_seconds = now_seconds();
    }

    pub fn get_elapsed_seconds(&self) -> u64 {
        if self.is_paused {
            0
        } else {
            now_seconds() - self.phase_started_at_seconds
        }
    }

    pub fn start_focus(&mut self) {
        self.update_times();
        self.unpause();
        self.phase = Phase::Focusing;
    }

    pub fn start_break(&mut self) {
        self.update_times();
        self.unpause();
        self.phase = Phase::Breaking;
    }

    pub fn toggle_phase(&mut self) {
        match self.phase {
            Phase::Idle | Phase::Breaking => self.start_focus(),
            Phase::Focusing => self.start_break(),
        }
    }

    pub fn pause(&mut self) {
        if self.is_paused {
            return;
        }

        self.update_times();
        self.is_paused = true;
    }

    pub fn unpause(&mut self) {
        if !self.is_paused {
            return;
        }

        self.phase_started_at_seconds = now_seconds();
        self.is_paused = false;
    }

    pub fn toggle_pause(&mut self) {
        if self.is_paused {
            self.unpause();
        } else {
            self.pause();
        }
    }

    pub fn get_stopwatch_status(&self) -> StopwatchStatus {
        StopwatchStatus {
            is_paused: self.is_paused,
            phase: self.phase.clone(),
            focused_seconds: self.total_focused_seconds,
            breaked_seconds: self.total_breaked_seconds,
            balance: calculate_balance(self),
        }
    }
}

pub fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Current system time should be after 1970-01-01 00:00:00 UTC")
        .as_secs()
}

fn to_minutes_u64(seconds: u64) -> u64 {
    seconds / 60
}

fn to_minutes_i128(seconds: i128) -> i128 {
    seconds / 60
}

impl StopwatchStatus {
    pub fn to_minutes(&self) -> Self {
        Self {
            is_paused: self.is_paused,
            phase: self.phase.clone(),
            focused_seconds: to_minutes_u64(self.focused_seconds),
            breaked_seconds: to_minutes_u64(self.breaked_seconds),
            balance: to_minutes_i128(self.balance),
        }
    }
}
