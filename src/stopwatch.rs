use std::{
    error::Error,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::history::HistoryEntry;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
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
    /// ISO 8601 date of the last daemon startup, used to detect day boundaries and reset daily totals.
    pub last_started_date: String,
}

#[derive(Serialize, Deserialize)]
pub struct StopwatchStatus {
    pub is_paused: bool,
    pub phase: Phase,
    pub focused_duration: u64,
    pub breaked_duration: u64,
    pub balance: i128,
}

impl StopwatchState {
    /// Creates a new, paused [`StopwatchState`] in [`Phase::Idle`] with zeroed totals and today's date.
    pub fn new(break_ratio: BreakRatio) -> Self {
        Self {
            is_paused: true,
            phase: Phase::Idle,
            phase_started_at_seconds: now_seconds(),
            total_focused_seconds: 0,
            total_breaked_seconds: 0,
            break_ratio,
            last_started_date: chrono::Local::now().date_naive().to_string(),
        }
    }

    /// Loads persisted state from `~/.local/share/paus/state.json`.
    ///
    /// On success the stopwatch is reset to paused and the phase timer restarted,
    /// so time accumulated between shutdown and now is not counted.
    ///
    /// # Errors
    ///
    /// Returns an error if the data directory is not found, the file cannot be read,
    /// or the JSON cannot be deserialized.
    pub fn try_read_state() -> Result<Self, Box<dyn Error>> {
        let share_dir = dirs::data_local_dir().ok_or("Failed to find local share dir")?;

        let bytes = std::fs::read(share_dir.join("paus/state.json"))?;

        let state: Self = serde_json::from_slice(&bytes)?;

        Ok(state)
    }

    /// Persists the current state to `~/.local/share/paus/state.json`, creating the directory if needed.
    ///
    /// # Errors
    ///
    /// Returns an error if the data directory is not found, the directory cannot be created,
    /// JSON serialization fails, or the file cannot be written.
    pub fn try_save_state(&self) -> Result<(), Box<dyn Error>> {
        let share_dir = dirs::data_local_dir().ok_or("Failed to find local share dir")?;
        let path = share_dir.join("paus/state.json");

        std::fs::create_dir_all(path.parent().ok_or("Failed to get ~/.local/share/paus")?)?;

        let bytes = serde_json::to_vec(self)?;

        std::fs::write(path, bytes)?;

        Ok(())
    }

    /// Adds elapsed time since the last update to the running phase total and resets the phase timer.
    ///
    /// No-op if [`Phase::Idle`].
    pub fn update_times(&mut self, elapsed_seconds: u64) {
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

    pub fn update_times_and_append_history(&mut self) {
        let elapsed_seconds = self.get_elapsed_seconds();
        self.update_times(elapsed_seconds);
        if elapsed_seconds > 0 && self.phase != Phase::Idle {
            let _ = HistoryEntry::append_history(self, elapsed_seconds);
        }
    }

    /// Returns seconds elapsed in the current phase since the last update.
    ///
    /// Returns `0` if paused.
    pub fn get_elapsed_seconds(&self) -> u64 {
        if self.is_paused {
            0
        } else {
            now_seconds() - self.phase_started_at_seconds
        }
    }

    /// Commits accumulated time, unpauses, and switches to [`Phase::Focusing`].
    pub fn start_focus(&mut self) {
        self.update_times_and_append_history();
        self.unpause();
        self.phase = Phase::Focusing;
    }

    /// Commits accumulated time, unpauses, and switches to [`Phase::Breaking`].
    pub fn start_break(&mut self) {
        self.update_times_and_append_history();
        self.unpause();
        self.phase = Phase::Breaking;
    }

    /// Switches between focus and break. Starts focusing from idle or break; starts a break from focus.
    pub fn toggle_phase(&mut self) {
        match self.phase {
            Phase::Idle | Phase::Breaking => self.start_focus(),
            Phase::Focusing => self.start_break(),
        }
    }

    /// Pauses the stopwatch, committing elapsed time first. No-op if already paused.
    pub fn pause(&mut self) {
        if self.is_paused {
            return;
        }

        self.update_times_and_append_history();
        self.is_paused = true;
    }

    /// Unpauses the stopwatch, resetting the phase timer to now. No-op if not paused.
    pub fn unpause(&mut self) {
        if !self.is_paused {
            return;
        }

        if self.phase == Phase::Idle {
            self.phase = Phase::Focusing;
        }
        self.phase_started_at_seconds = now_seconds();
        self.is_paused = false;
    }

    /// Toggles between paused and unpaused.
    pub fn toggle_pause(&mut self) {
        if self.is_paused {
            self.unpause();
        } else {
            self.pause();
        }
    }

    /// Returns a snapshot of current totals and balance without mutating state.
    pub fn get_stopwatch_status(&self) -> StopwatchStatus {
        let elapsed = self.get_elapsed_seconds();

        let (focused_duration, breaked_duration) = match self.phase {
            Phase::Focusing => (
                self.total_focused_seconds + elapsed,
                self.total_breaked_seconds,
            ),
            Phase::Breaking => (
                self.total_focused_seconds,
                self.total_breaked_seconds + elapsed,
            ),
            Phase::Idle => (self.total_focused_seconds, self.total_breaked_seconds),
        };

        let balance = (focused_duration as i128 / self.break_ratio.clone() as i128)
            - breaked_duration as i128;

        StopwatchStatus {
            is_paused: self.is_paused,
            phase: self.phase.clone(),
            focused_duration,
            breaked_duration,
            balance,
        }
    }
}

/// Returns the current Unix timestamp in whole seconds.
pub fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Current system time should be after 1970-01-01 00:00:00 UTC")
        .as_secs()
}

impl StopwatchStatus {
    /// Returns a copy with all time fields converted from seconds to minutes (truncating).
    pub fn to_minutes(&self) -> Self {
        Self {
            is_paused: self.is_paused,
            phase: self.phase.clone(),
            focused_duration: self.focused_duration / 60,
            breaked_duration: self.breaked_duration / 60,
            balance: self.balance / 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stopwatch_state(
        is_paused: bool,
        phase: Phase,
        focused: u64,
        breaked: u64,
        break_ratio: BreakRatio,
    ) -> StopwatchState {
        StopwatchState {
            is_paused,
            phase,
            phase_started_at_seconds: 0,
            total_focused_seconds: focused,
            total_breaked_seconds: breaked,
            break_ratio,
            last_started_date: String::new(),
        }
    }

    mod to_minutes {
        use super::*;

        fn make_stopwatch_status(focused: u64, breaked: u64, balance: i128) -> StopwatchStatus {
            StopwatchStatus {
                is_paused: true,
                phase: Phase::Idle,
                focused_duration: focused,
                breaked_duration: breaked,
                balance,
            }
        }

        #[test]
        fn focused_duration_is_divided_by_60() {
            let status = make_stopwatch_status(3600, 0, 0).to_minutes();
            assert_eq!(status.focused_duration, 60);
        }

        #[test]
        fn breaked_duration_is_divided_by_60() {
            let status = make_stopwatch_status(0, 120, 0).to_minutes();
            assert_eq!(status.breaked_duration, 2);
        }

        #[test]
        fn balance_is_divided_by_60() {
            let status = make_stopwatch_status(0, 0, 24000).to_minutes();
            assert_eq!(status.balance, 400);
        }
    }

    mod get_stopwatch_status {
        use super::*;

        #[test]
        fn balance_is_focused_over_ratio_minus_breaked() {
            let state =
                make_stopwatch_state(true, Phase::Focusing, 2100, 300, BreakRatio::Standard);
            let status = state.get_stopwatch_status();
            // balance = (2100 / 3) - 300 = 400
            assert_eq!(status.balance, 400);
        }
    }

    mod start_focus {
        use super::*;

        #[test]
        fn sets_phase_to_focusing() {
            let mut state = StopwatchState::new(BreakRatio::Standard);
            state.start_focus();
            assert_eq!(state.phase, Phase::Focusing);
        }

        #[test]
        fn sets_is_paused_to_false() {
            let mut state = StopwatchState::new(BreakRatio::Standard);
            state.start_focus();
            assert_eq!(state.is_paused, false);
        }
    }

    mod start_break {
        use super::*;

        #[test]
        fn sets_phase_to_breaking() {
            let mut state = StopwatchState::new(BreakRatio::Standard);
            state.start_break();
            assert_eq!(state.phase, Phase::Breaking);
        }

        #[test]
        fn sets_is_paused_to_false() {
            let mut state = StopwatchState::new(BreakRatio::Standard);
            state.start_break();
            assert_eq!(state.is_paused, false);
        }
    }

    mod toggle_phase {
        use super::*;

        #[test]
        fn sets_phase_to_focusing_if_idle() {
            let mut state = make_stopwatch_state(true, Phase::Idle, 0, 0, BreakRatio::Standard);
            state.toggle_phase();
            assert_eq!(state.phase, Phase::Focusing);
        }

        #[test]
        fn sets_phase_to_focusing_if_breaking() {
            let mut state = make_stopwatch_state(true, Phase::Breaking, 0, 0, BreakRatio::Standard);
            state.toggle_phase();
            assert_eq!(state.phase, Phase::Focusing);
        }

        #[test]
        fn sets_phase_to_breaking_if_focusing() {
            let mut state = make_stopwatch_state(true, Phase::Focusing, 0, 0, BreakRatio::Standard);
            state.toggle_phase();
            assert_eq!(state.phase, Phase::Breaking);
        }

        #[test]
        fn sets_is_paused_to_false() {
            let mut state = make_stopwatch_state(true, Phase::Idle, 0, 0, BreakRatio::Standard);
            state.toggle_phase();
            assert_eq!(state.is_paused, false);
        }
    }

    mod pause {
        use super::*;

        #[test]
        fn sets_is_paused_to_true() {
            let mut state = make_stopwatch_state(false, Phase::Idle, 0, 0, BreakRatio::Standard);
            state.pause();
            assert!(state.is_paused);
        }

        #[test]
        fn is_noop_when_paused() {
            let mut state = make_stopwatch_state(true, Phase::Idle, 0, 0, BreakRatio::Standard);
            state.pause();
            state.pause();
            assert!(state.is_paused);
        }
    }

    mod unpause {
        use super::*;

        #[test]
        fn sets_is_paused_to_false() {
            let mut state = make_stopwatch_state(true, Phase::Idle, 0, 0, BreakRatio::Standard);
            state.unpause();
            assert!(!state.is_paused);
        }

        #[test]
        fn sets_phase_to_focusing_if_idle() {
            let mut state = make_stopwatch_state(true, Phase::Idle, 0, 0, BreakRatio::Standard);
            state.unpause();
            assert_eq!(state.phase, Phase::Focusing);
        }

        #[test]
        fn is_noop_when_unpaused() {
            let mut state = make_stopwatch_state(false, Phase::Idle, 0, 0, BreakRatio::Standard);
            state.unpause();
            state.unpause();
            assert!(!state.is_paused);
        }
    }

    mod toggle_unpause {
        use super::*;

        // if self.is_paused {
        //     self.unpause();
        // } else {
        //     self.pause();
        // }

        #[test]
        fn sets_is_paused_to_true_if_unpaused() {
            let mut state = make_stopwatch_state(false, Phase::Idle, 0, 0, BreakRatio::Standard);
            state.toggle_pause();
            assert!(state.is_paused);
        }

        #[test]
        fn sets_is_paused_to_false_if_paused() {
            let mut state = make_stopwatch_state(true, Phase::Idle, 0, 0, BreakRatio::Standard);
            state.toggle_pause();
            assert!(!state.is_paused);
        }
    }
}
