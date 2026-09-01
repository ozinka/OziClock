//! Versioned JSON settings and local file locations for OziClock.

use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::PathBuf};

pub use oziclock_domain::Clock as ClockSettings;

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
    #[serde(default)]
    pub launch_at_login: bool,
    pub show_seconds: bool,
    #[serde(default)]
    pub compact_mode: bool,
    #[serde(default)]
    pub show_rulers: bool,
    #[serde(default = "default_clock_scale")]
    pub clock_scale: f64,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,
    #[serde(default)]
    pub soft_clock_style: bool,
    #[serde(default = "default_border_color")]
    pub border_color: String,
    #[serde(default)]
    pub non_main_dimming: f64,
    #[serde(default = "default_calendar_light_theme")]
    pub calendar_light_theme: bool,
    #[serde(default = "default_calendar_monday_first")]
    pub calendar_monday_first: bool,
    #[serde(default)]
    pub calendar_hour_range: u8,
    #[serde(default = "default_settings_window_width")]
    pub settings_window_width: f64,
    #[serde(default = "default_settings_window_height")]
    pub settings_window_height: f64,
    pub clocks_settings: Vec<ClockSettings>,
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

fn default_corner_radius() -> f64 {
    12.0
}

fn default_border_color() -> String {
    "#000000".to_owned()
}

fn default_settings_window_height() -> f64 {
    700.0
}

fn default_calendar_light_theme() -> bool {
    true
}

fn default_calendar_monday_first() -> bool {
    true
}

#[cfg(target_os = "macos")]
fn macos_application_support_directory() -> io::Result<PathBuf> {
    let home = env::var_os("HOME").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "HOME is not set; cannot locate macOS Application Support",
        )
    })?;

    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("OziClock"))
}

#[cfg(target_os = "macos")]
fn legacy_macos_settings_path(executable: &std::path::Path) -> Option<PathBuf> {
    executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .and_then(|bundle| bundle.parent())
        .map(|directory| directory.join("settings.json"))
}

#[cfg(any(target_os = "macos", test))]
fn migrate_legacy_settings(
    target: &std::path::Path,
    legacy: Option<&std::path::Path>,
) -> io::Result<bool> {
    if target.exists() {
        return Ok(false);
    }

    let Some(legacy) = legacy.filter(|path| path.is_file()) else {
        return Ok(false);
    };
    let parent = target.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "settings path has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;
    fs::copy(legacy, target)?;
    Ok(true)
}

/// Returns the per-user settings path on macOS and the portable path elsewhere.
pub fn settings_path() -> io::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        return Ok(macos_application_support_directory()?.join("settings.json"));
    }

    #[cfg(not(target_os = "macos"))]
    let executable = env::current_exe()?;
    #[cfg(not(target_os = "macos"))]
    let directory = executable.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "running executable has no parent directory",
        )
    })?;

    #[cfg(not(target_os = "macos"))]
    {
        Ok(directory.join("settings.json"))
    }
}

/// Loads settings, creating the default JSON file on first launch.
pub fn load_or_initialize() -> Result<AppSettings, Box<dyn std::error::Error>> {
    let path = settings_path()?;

    if !path.exists() {
        #[cfg(target_os = "macos")]
        migrate_legacy_settings(
            &path,
            legacy_macos_settings_path(&env::current_exe()?).as_deref(),
        )?;

        if path.exists() {
            let content = fs::read_to_string(path)?;
            return Ok(serde_json::from_str(&content)?);
        }

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
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn bundled_defaults_are_valid_and_have_a_main_clock() {
        let settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();

        assert!(!settings.clocks_settings.is_empty());
        assert!(settings.clocks_settings.iter().any(|clock| clock.is_main));
        assert_eq!(settings.corner_radius, 12.0);
        assert!(!settings.soft_clock_style);
        assert_eq!(settings.border_color, "#000000");
        assert_eq!(settings.non_main_dimming, 0.0);
    }

    #[test]
    fn migrates_legacy_settings_without_overwriting_current_settings() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("oziclock-storage-{unique}"));
        let legacy = root.join("legacy/settings.json");
        let target = root.join("current/settings.json");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, "legacy").unwrap();

        assert!(migrate_legacy_settings(&target, Some(&legacy)).unwrap());
        assert_eq!(fs::read_to_string(&target).unwrap(), "legacy");

        fs::write(&legacy, "changed").unwrap();
        assert!(!migrate_legacy_settings(&target, Some(&legacy)).unwrap());
        assert_eq!(fs::read_to_string(&target).unwrap(), "legacy");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn appearance_defaults_are_added_to_older_settings() {
        let mut legacy: serde_json::Value = serde_json::from_str(DEFAULT_SETTINGS).unwrap();
        let document = legacy.as_object_mut().unwrap();
        document.remove("CornerRadius");
        document.remove("SoftClockStyle");
        document.remove("BorderColor");
        document.remove("NonMainDimming");
        let settings: AppSettings = serde_json::from_value(legacy).unwrap();

        assert_eq!(settings.corner_radius, 12.0);
        assert!(!settings.soft_clock_style);
        assert_eq!(settings.border_color, "#000000");
        assert_eq!(settings.non_main_dimming, 0.0);
    }
}
