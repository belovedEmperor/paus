use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Clone, Serialize)]
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

fn to_minutes(seconds: i128) -> i128 {
    seconds / 60
}

impl StopwatchState {
    pub fn new(
        total_focused_seconds: u64,
        total_breaked_seconds: u64,
        break_ratio: BreakRatio,
    ) -> Self {
        Self {
            is_paused: true,
            phase: Phase::Idle,
            phase_started_at_seconds: now_seconds(),
            total_focused_seconds,
            total_breaked_seconds,
            break_ratio,
        }
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
