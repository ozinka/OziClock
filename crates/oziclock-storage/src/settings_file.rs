//! Fixed per-user paths and one-time import of pre-v2 local stores.

use super::{AppSettings, DEFAULT_SETTINGS, SETTINGS_FILE_NAME, SyncState, save_at};
use serde::Deserialize;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

const CURRENT_SCHEMA_VERSION: u32 = 2;

/// The local document's path is derived from the user environment, never from
/// settings or the executable's location. Only migration inspects old locations.
pub fn settings_path() -> io::Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        Ok(windows_settings_path(&required_home("USERPROFILE")?))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(macos_settings_path(&required_home("HOME")?))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let config = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
        if let Some(config) = config.as_deref().filter(|path| path.is_absolute()) {
            return Ok(config.join("oziclock").join(SETTINGS_FILE_NAME));
        }
        Ok(linux_settings_path(
            &required_home("HOME")?,
            config.as_deref(),
        ))
    }
}

fn required_home(variable: &str) -> io::Result<PathBuf> {
    env::var_os(variable)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{variable} must name an absolute user home directory"),
            )
        })
}

#[cfg(any(target_os = "windows", test))]
fn windows_settings_path(home: &Path) -> PathBuf {
    home.join(".oziclock").join(SETTINGS_FILE_NAME)
}

#[cfg(any(target_os = "macos", test))]
fn macos_settings_path(home: &Path) -> PathBuf {
    home.join("Library/Application Support/OziClock")
        .join(SETTINGS_FILE_NAME)
}

#[cfg(any(not(any(target_os = "windows", target_os = "macos")), test))]
fn linux_settings_path(home: &Path, config: Option<&Path>) -> PathBuf {
    config
        .filter(|path| path.is_absolute())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".config"))
        .join("oziclock")
        .join(SETTINGS_FILE_NAME)
}

/// Imports a legacy store once and persists Sync state in the same document.
/// Legacy files are retained, but are never read again after migration succeeds.
pub fn load_or_initialize() -> Result<AppSettings, Box<dyn std::error::Error>> {
    let target = settings_path()?;
    let executable = env::current_exe()?;
    #[cfg(target_os = "macos")]
    let legacy = target.clone();
    #[cfg(not(target_os = "macos"))]
    let legacy = executable.with_file_name(SETTINGS_FILE_NAME);
    let bundle_legacy = executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|ext| ext == "app"))
        .and_then(|bundle| bundle.parent())
        .map(|directory| directory.join(SETTINGS_FILE_NAME));
    load_at(&target, &legacy, bundle_legacy.as_deref())
}

fn read_settings(path: &Path) -> Result<AppSettings, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn load_at(
    target: &Path,
    legacy: &Path,
    bundle_legacy: Option<&Path>,
) -> Result<AppSettings, Box<dyn std::error::Error>> {
    let existing = match read_settings(target) {
        Ok(settings) => Some(settings),
        Err(_) if !target.try_exists()? => None,
        Err(error) => return Err(error),
    };
    if let Some(settings) = &existing
        && settings.schema_version >= CURRENT_SCHEMA_VERSION
    {
        return Ok(existing.unwrap());
    }

    // macOS already uses the fixed folder, but a v1 recovery copy may coexist
    // there with a bootstrap pointing to the actual current document.
    let importing_legacy = existing.is_none() || target == legacy;
    let mut settings = if importing_legacy {
        legacy_custom_settings(legacy)
            .or_else(|| existing.clone())
            .or_else(|| read_settings(legacy).ok())
            .or_else(|| bundle_legacy.and_then(|path| read_settings(path).ok()))
            .unwrap_or(serde_json::from_str(DEFAULT_SETTINGS)?)
    } else {
        existing.unwrap()
    };
    let sync_path =
        if importing_legacy { legacy } else { target }.with_file_name("sync-state.json");
    // Preserve an already embedded Sync section; import the old sidecar only
    // when there is no configured state in the chosen document.
    if settings.sync == SyncState::default()
        && let Ok(content) = fs::read_to_string(sync_path)
        && let Ok(sync) = serde_json::from_str(&content)
    {
        settings.sync = sync;
    }
    settings.schema_version = settings.schema_version.max(CURRENT_SCHEMA_VERSION);
    save_at(&settings, target)?;
    Ok(settings)
}

fn legacy_custom_settings(legacy: &Path) -> Option<AppSettings> {
    #[derive(Deserialize)]
    struct LegacyLocation {
        custom_directory: Option<PathBuf>,
    }
    let content = fs::read_to_string(legacy.with_file_name("settings-location.json")).ok()?;
    let location: LegacyLocation = serde_json::from_str(&content).ok()?;
    read_settings(&location.custom_directory?.join(SETTINGS_FILE_NAME)).ok()
}

#[cfg(test)]
mod tests {
    use super::super::SyncGroups;
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static SEQUENCE: AtomicU64 = AtomicU64::new(0);
            let root = env::temp_dir().join(format!(
                "oziclock-fixed-store-{}-{}",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
        fn path(&self, relative: &str) -> PathBuf {
            self.0.join(relative)
        }
        fn legacy(&self, relative: &str, opacity: f64) -> PathBuf {
            let mut settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();
            settings.schema_version = 1;
            settings.opacity = opacity;
            let path = self.path(relative);
            save_at(&settings, &path).unwrap();
            path
        }
        fn bootstrap(&self, legacy: &Path, directory: &Path) {
            fs::write(
                legacy.with_file_name("settings-location.json"),
                serde_json::json!({"custom_directory": directory}).to_string(),
            )
            .unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn set_12_fixed_paths_use_home_and_absolute_xdg_config() {
        let home = env::temp_dir().join("user-home");
        let xdg = env::temp_dir().join("custom-config");
        assert_eq!(
            windows_settings_path(&home),
            home.join(".oziclock/settings.json")
        );
        assert_eq!(
            macos_settings_path(&home),
            home.join("Library/Application Support/OziClock/settings.json")
        );
        assert_eq!(
            linux_settings_path(&home, None),
            home.join(".config/oziclock/settings.json")
        );
        assert_eq!(
            linux_settings_path(&home, Some(Path::new(""))),
            linux_settings_path(&home, None)
        );
        assert_eq!(
            linux_settings_path(&home, Some(Path::new("relative"))),
            linux_settings_path(&home, None)
        );
        assert_eq!(
            linux_settings_path(&home, Some(&xdg)),
            xdg.join("oziclock/settings.json")
        );
    }

    #[test]
    fn set_12_first_launch_creates_only_one_local_document() {
        let f = Fixture::new();
        let target = f.path("new/settings.json");
        let settings = load_at(&target, &f.path("old/settings.json"), None).unwrap();
        assert_eq!(settings.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(settings.sync, SyncState::default());
        assert_eq!(fs::read_dir(target.parent().unwrap()).unwrap().count(), 1);
        assert!(!f.path("old").exists());
    }

    #[test]
    fn set_12_imports_custom_document_and_sync_once_without_changing_legacy_files() {
        let f = Fixture::new();
        let legacy = f.legacy("old/settings.json", 0.4);
        let custom = f.legacy("custom/settings.json", 0.8);
        f.bootstrap(&legacy, custom.parent().unwrap());
        let sync = SyncState {
            profile_directory: Some(f.path("profile")),
            groups: SyncGroups {
                planner: true,
                ..SyncGroups::default()
            },
            last_common_revision: Some(9),
            ..SyncState::default()
        };
        fs::write(
            legacy.with_file_name("sync-state.json"),
            serde_json::to_string(&sync).unwrap(),
        )
        .unwrap();
        let target = f.path("new/settings.json");
        let settings = load_at(&target, &legacy, None).unwrap();
        assert_eq!(settings.opacity, 0.8);
        assert_eq!(settings.sync, sync);
        assert_eq!(read_settings(&target).unwrap().sync, sync);
        assert_eq!(read_settings(&legacy).unwrap().opacity, 0.4);
        f.legacy("custom/settings.json", 0.2);
        assert_eq!(load_at(&target, &legacy, None).unwrap().opacity, 0.8);
        assert_eq!(fs::read_dir(target.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn set_12_existing_fixed_file_wins_over_legacy_even_before_schema_upgrade() {
        let f = Fixture::new();
        let target = f.legacy("new/settings.json", 0.9);
        let legacy = f.legacy("old/settings.json", 0.3);
        let custom = f.legacy("custom/settings.json", 0.1);
        f.bootstrap(&legacy, custom.parent().unwrap());
        assert_eq!(load_at(&target, &legacy, None).unwrap().opacity, 0.9);
    }

    #[test]
    fn set_12_macos_imports_custom_store_over_old_recovery_copy_once() {
        let f = Fixture::new();
        let target = f.legacy("support/settings.json", 0.4);
        let custom = f.legacy("custom/settings.json", 0.8);
        f.bootstrap(&target, custom.parent().unwrap());
        assert_eq!(load_at(&target, &target, None).unwrap().opacity, 0.8);
        f.legacy("custom/settings.json", 0.2);
        assert_eq!(load_at(&target, &target, None).unwrap().opacity, 0.8);
    }

    #[test]
    fn set_12_unavailable_or_invalid_custom_store_uses_legacy_recovery() {
        for content in [None, Some("not json")] {
            let f = Fixture::new();
            let legacy = f.legacy("old/settings.json", 0.6);
            let custom = f.path("custom");
            fs::create_dir_all(&custom).unwrap();
            if let Some(content) = content {
                fs::write(custom.join(SETTINGS_FILE_NAME), content).unwrap();
            }
            f.bootstrap(&legacy, &custom);
            assert_eq!(
                load_at(&f.path("new/settings.json"), &legacy, None)
                    .unwrap()
                    .opacity,
                0.6
            );
        }
    }

    #[test]
    fn set_12_invalid_existing_fixed_document_is_not_replaced_by_legacy() {
        let f = Fixture::new();
        let target = f.path("settings.json");
        fs::write(&target, "invalid").unwrap();
        let legacy = f.legacy("old/settings.json", 0.6);
        assert!(load_at(&target, &legacy, None).is_err());
        assert_eq!(fs::read_to_string(target).unwrap(), "invalid");
    }

    #[test]
    fn set_03_macos_legacy_bundle_settings_are_still_imported() {
        let f = Fixture::new();
        let bundle = f.legacy("bundle/settings.json", 0.7);
        let target = f.path("support/settings.json");
        assert_eq!(
            load_at(&target, &target, Some(&bundle)).unwrap().opacity,
            0.7
        );
    }
}
