//! Versioned JSON settings and local file locations for OziClock.

use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::PathBuf};

const DEFAULT_SETTINGS: &str = include_str!("../assets/default_settings.json");

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AppSettings {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub main_wnd_left: f64,
    pub main_wnd_top: f64,
    pub opacity: f64,
    pub top_most: bool,
    pub show_in_task_bar: bool,
    pub show_seconds: bool,
    #[serde(default)]
    pub compact_mode: bool,
    #[serde(default)]
    pub show_rulers: bool,
    #[serde(default = "default_clock_scale")]
    pub clock_scale: f64,
    #[serde(default = "default_settings_window_width")]
    pub settings_window_width: f64,
    #[serde(default = "default_settings_window_height")]
    pub settings_window_height: f64,
    pub clocks_settings: Vec<ClockSettings>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ClockSettings {
    pub label: String,
    pub time_zone: String,
    pub color: String,
    #[serde(default)]
    pub is_main: bool,
}

fn default_schema_version() -> u32 {
    1
}

fn default_settings_window_width() -> f64 {
    760.0
}

fn default_clock_scale() -> f64 {
    1.0
}

fn default_settings_window_height() -> f64 {
    510.0
}

/// Returns the portable settings path beside the running executable.
pub fn settings_path() -> io::Result<PathBuf> {
    let executable = env::current_exe()?;
    #[cfg(target_os = "macos")]
    if let Some(bundle) = executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
    {
        let directory = bundle.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "application bundle has no parent directory",
            )
        })?;
        return Ok(directory.join("settings.json"));
    }
    let directory = executable.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "running executable has no parent directory",
        )
    })?;

    Ok(directory.join("settings.json"))
}

/// Loads settings, creating the default JSON file on first launch.
pub fn load_or_initialize() -> Result<AppSettings, Box<dyn std::error::Error>> {
    let path = settings_path()?;

    if !path.exists() {
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "settings path has no parent directory",
            )
        })?;
        fs::create_dir_all(parent)?;
        fs::write(&path, DEFAULT_SETTINGS)?;
    }

    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

/// Persists settings beside the executable.
pub fn save(settings: &AppSettings) -> Result<(), Box<dyn std::error::Error>> {
    let content = serde_json::to_string_pretty(settings)?;
    fs::write(settings_path()?, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_defaults_are_valid_and_have_a_main_clock() {
        let settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();

        assert!(!settings.clocks_settings.is_empty());
        assert!(settings.clocks_settings.iter().any(|clock| clock.is_main));
    }
}
