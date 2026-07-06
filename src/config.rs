use std::{fs, path::PathBuf};

use serde::Deserialize;

use crate::stopwatch::BreakRatio;

/// User configuration for `paus`, loaded from [`Config::path`] at daemon startup.
///
/// All fields are optional in the config file — missing fields fall back to [`Default`].
/// Fields use variant names as strings, e.g. `{ "break_ratio": "Standard" }`.
#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    /// How much break time is earned per second of focus.
    ///
    /// Serialized as the variant name, e.g. `"Standard"` (not the integer discriminant).
    /// Defaults to [`BreakRatio::Standard`].
    pub break_ratio: BreakRatio,
    /// Where the state directory is.
    pub data_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_local_dir()
            .or_else(|| dirs::home_dir().map(|home_dir| home_dir.join(".local/share")))
            .expect("No home directory found")
            .join("paus");
        Self {
            break_ratio: BreakRatio::Standard,
            data_dir,
        }
    }
}

impl Config {
    /// Returns the expected config file path: `$XDG_CONFIG_HOME/paus/config.json`.
    ///
    /// Returns `None` if the platform config directory cannot be determined.
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|directory| directory.join("paus").join("config.json"))
    }

    /// Loads config from [`Config::path`], falling back to [`Default`] on any failure.
    ///
    /// Silently returns defaults if the config directory is unavailable, the file does not
    /// exist, cannot be read, or contains invalid JSON. A missing file is not an error.
    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };

        let Ok(config_string) = fs::read_to_string(&path) else {
            return Self::default();
        };

        serde_json::from_str(&config_string).unwrap_or_default()
    }
}
