//! Versioned JSON settings and local file locations for OziClock.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub use oziclock_domain::{
    Alarm, AlarmOccurrenceStatus, AlarmReceipt, AlarmSchedule, AlarmSnooze, AlertRule,
    Clock as ClockSettings, Event, EventReceipt, EventRecurrence, EventTime, Planner, PlannerId,
    Reminder, ReminderRecurrence, ReminderSchedule, Stopwatch, StopwatchState, Task, TaskStatus,
    Timer as PlannerTimer, TimerState,
};

const DEFAULT_SETTINGS: &str = include_str!("../assets/default_settings.json");
const SETTINGS_FILE_NAME: &str = "settings.json";
const LOCATION_BOOTSTRAP_FILE_NAME: &str = "settings-location.json";
const SYNC_PROFILE_FILE_NAME: &str = "oziclock-sync.json";
const SYNC_STATE_FILE_NAME: &str = "sync-state.json";
static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsLocation {
    pub directory: PathBuf,
    pub uses_default: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct SettingsLocationBootstrap {
    custom_directory: Option<PathBuf>,
}

/// Portable data groups that a device can independently include in a profile.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SyncGroups {
    pub planner: bool,
    pub clocks: bool,
    pub appearance: bool,
}

/// Local-only state for a provider-managed sync profile.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SyncState {
    pub profile_directory: Option<PathBuf>,
    #[serde(default)]
    pub groups: SyncGroups,
    #[serde(default)]
    pub last_common_revision: Option<u64>,
    #[serde(default)]
    pub unresolved_conflict: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PortablePlanner {
    events: Vec<Event>,
    tasks: Vec<Task>,
    reminders: Vec<Reminder>,
}

/// The versioned document shared through a cloud-provider desktop folder.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SyncProfile {
    #[serde(default = "default_sync_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    planner: Option<PortablePlanner>,
    #[serde(default)]
    clocks: Option<Vec<ClockSettings>>,
    #[serde(default)]
    appearance: Option<PlannerAppearance>,
}

/// A read-only summary shown before a one-time profile transfer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncPreview {
    pub profile_exists: bool,
    pub profile_revision: Option<u64>,
    pub selected_groups: SyncGroups,
    pub available_groups: SyncGroups,
}

impl Default for SyncProfile {
    fn default() -> Self {
        Self {
            schema_version: default_sync_schema_version(),
            revision: 0,
            planner: None,
            clocks: None,
            appearance: None,
        }
    }
}

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
    #[serde(default = "default_calendar_indicator_filter")]
    pub calendar_show_events: bool,
    #[serde(default = "default_calendar_indicator_filter")]
    pub calendar_show_reminders: bool,
    #[serde(default = "default_calendar_indicator_filter")]
    pub calendar_show_tasks: bool,
    #[serde(default = "default_calendar_indicator_filter")]
    pub calendar_show_alarms: bool,
    #[serde(default = "default_alert_sound_duration_seconds")]
    pub alert_sound_duration_seconds: u8,
    #[serde(default = "default_settings_window_width")]
    pub settings_window_width: f64,
    #[serde(default = "default_settings_window_height")]
    pub settings_window_height: f64,
    #[serde(default)]
    pub planner: Planner,
    #[serde(default)]
    pub planner_appearance: PlannerAppearance,
    #[serde(default)]
    pub timer_draft_days: u16,
    #[serde(default)]
    pub timer_draft_hours: u8,
    #[serde(default = "default_timer_draft_minutes")]
    pub timer_draft_minutes: u8,
    #[serde(default)]
    pub timer_draft_seconds: u8,
    pub clocks_settings: Vec<ClockSettings>,
}

/// Independent Planner appearance; missing fields preserve the pre-settings theme.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, rename_all = "PascalCase")]
pub struct PlannerAppearance {
    pub light_theme: bool,
    pub follow_main_clock: bool,
    pub custom_accent: String,
    pub use_accent_color: bool,
    pub alarm_color: String,
    pub timer_color: String,
    pub reminder_color: String,
    pub event_color: String,
    pub task_color: String,
}

impl Default for PlannerAppearance {
    fn default() -> Self {
        Self {
            light_theme: false,
            follow_main_clock: true,
            custom_accent: "#77D7CB".into(),
            use_accent_color: true,
            alarm_color: "#77D7CB".into(),
            timer_color: "#77D7CB".into(),
            reminder_color: "#77D7CB".into(),
            event_color: "#B99655".into(),
            task_color: "#547D78".into(),
        }
    }
}

fn default_schema_version() -> u32 {
    1
}

fn default_sync_schema_version() -> u32 {
    1
}

fn default_timer_draft_minutes() -> u8 {
    5
}

fn default_settings_window_width() -> f64 {
    710.0
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
    672.0
}

fn default_calendar_light_theme() -> bool {
    true
}

fn default_calendar_monday_first() -> bool {
    true
}

fn default_calendar_indicator_filter() -> bool {
    true
}

fn default_alert_sound_duration_seconds() -> u8 {
    20
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
    let content = fs::read(legacy)?;
    write_atomically(target, &content)?;
    Ok(true)
}

/// Returns the platform-default settings path.
fn default_settings_path() -> io::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        Ok(macos_application_support_directory()?.join(SETTINGS_FILE_NAME))
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
        Ok(directory.join(SETTINGS_FILE_NAME))
    }
}

fn location_bootstrap_path(default_path: &Path) -> io::Result<PathBuf> {
    let parent = default_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "default settings path has no parent directory",
        )
    })?;
    Ok(parent.join(LOCATION_BOOTSTRAP_FILE_NAME))
}

fn sync_state_path(default_path: &Path) -> io::Result<PathBuf> {
    let parent = default_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "default settings path has no parent directory",
        )
    })?;
    Ok(parent.join(SYNC_STATE_FILE_NAME))
}

fn sync_profile_path(directory: &Path) -> PathBuf {
    directory.join(SYNC_PROFILE_FILE_NAME)
}

/// Loads the local-only configuration for the optional sync profile.
pub fn load_sync_state() -> Result<SyncState, Box<dyn std::error::Error>> {
    let default_path = default_settings_path()?;
    load_sync_state_at(&default_path)
}

fn load_sync_state_at(default_path: &Path) -> Result<SyncState, Box<dyn std::error::Error>> {
    let path = sync_state_path(default_path)?;
    if !path.exists() {
        return Ok(SyncState::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn save_sync_state_at(
    default_path: &Path,
    state: &SyncState,
) -> Result<(), Box<dyn std::error::Error>> {
    write_atomically(
        &sync_state_path(default_path)?,
        serde_json::to_string_pretty(state)?.as_bytes(),
    )?;
    Ok(())
}

/// Selects a provider-managed folder and the portable groups this device owns.
pub fn configure_sync_profile(
    directory: &Path,
    groups: SyncGroups,
) -> Result<SyncState, Box<dyn std::error::Error>> {
    let default_path = default_settings_path()?;
    configure_sync_profile_at(directory, groups, &default_path)
}

/// Summarizes a pending upload without modifying the profile or local settings.
pub fn preview_send_sync_profile() -> Result<SyncPreview, Box<dyn std::error::Error>> {
    let default_path = default_settings_path()?;
    let state = load_sync_state_at(&default_path)?;
    preview_sync_profile_at(&state)
}

/// Summarizes a pending download without modifying the profile or local settings.
pub fn preview_receive_sync_profile() -> Result<SyncPreview, Box<dyn std::error::Error>> {
    let default_path = default_settings_path()?;
    let state = load_sync_state_at(&default_path)?;
    preview_sync_profile_at(&state)
}

fn preview_sync_profile_at(state: &SyncState) -> Result<SyncPreview, Box<dyn std::error::Error>> {
    let directory = state.profile_directory.as_deref().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "configure a sync profile folder first",
        )
    })?;
    if !directory.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "the sync profile folder is unavailable",
        )
        .into());
    }
    let profile_path = sync_profile_path(directory);
    if !profile_path.exists() {
        return Ok(SyncPreview {
            profile_exists: false,
            profile_revision: None,
            selected_groups: state.groups,
            available_groups: SyncGroups::default(),
        });
    }
    let profile: SyncProfile = serde_json::from_str(&fs::read_to_string(profile_path)?)?;
    Ok(SyncPreview {
        profile_exists: true,
        profile_revision: Some(profile.revision),
        selected_groups: state.groups,
        available_groups: SyncGroups {
            planner: profile.planner.is_some(),
            clocks: profile.clocks.is_some(),
            appearance: profile.appearance.is_some(),
        },
    })
}

fn configure_sync_profile_at(
    directory: &Path,
    groups: SyncGroups,
    default_path: &Path,
) -> Result<SyncState, Box<dyn std::error::Error>> {
    if !directory.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "choose an existing folder for the sync profile",
        )
        .into());
    }
    if !groups.planner && !groups.clocks && !groups.appearance {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "select at least one group for the sync profile",
        )
        .into());
    }
    let profile_path = sync_profile_path(directory);
    if profile_path.exists() {
        let _: SyncProfile = serde_json::from_str(&fs::read_to_string(profile_path)?)?;
    }
    let state = SyncState {
        profile_directory: Some(directory.to_path_buf()),
        groups,
        last_common_revision: None,
        unresolved_conflict: None,
    };
    save_sync_state_at(default_path, &state)?;
    Ok(state)
}

/// Writes only this device's selected portable groups to its configured profile.
/// Existing unselected groups remain intact for other devices.
pub fn send_sync_profile(settings: &AppSettings) -> Result<SyncState, Box<dyn std::error::Error>> {
    let default_path = default_settings_path()?;
    let mut state = load_sync_state_at(&default_path)?;
    send_sync_profile_at(settings, &mut state, &default_path)?;
    Ok(state)
}

fn send_sync_profile_at(
    settings: &AppSettings,
    state: &mut SyncState,
    default_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = state.profile_directory.as_deref().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "configure a sync profile folder first",
        )
    })?;
    if !directory.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "the sync profile folder is unavailable",
        )
        .into());
    }
    let profile_path = sync_profile_path(directory);
    let mut profile = if profile_path.exists() {
        serde_json::from_str(&fs::read_to_string(&profile_path)?)?
    } else {
        SyncProfile::default()
    };
    if state.groups.planner {
        profile.planner = Some(PortablePlanner {
            events: settings.planner.events.clone(),
            tasks: settings.planner.tasks.clone(),
            reminders: settings.planner.reminders.clone(),
        });
    }
    if state.groups.clocks {
        profile.clocks = Some(settings.clocks_settings.clone());
    }
    if state.groups.appearance {
        profile.appearance = Some(settings.planner_appearance.clone());
    }
    profile.revision = profile.revision.saturating_add(1);
    write_atomically(
        &profile_path,
        serde_json::to_string_pretty(&profile)?.as_bytes(),
    )?;
    state.last_common_revision = Some(profile.revision);
    state.unresolved_conflict = None;
    save_sync_state_at(default_path, state)
}

/// Applies only this device's selected portable groups from the configured profile.
/// Local-only scheduling, delivery, timing, and window state are preserved.
pub fn receive_sync_profile(
    settings: &mut AppSettings,
) -> Result<SyncState, Box<dyn std::error::Error>> {
    let default_path = default_settings_path()?;
    let mut state = load_sync_state_at(&default_path)?;
    receive_sync_profile_at(settings, &mut state, &default_path)?;
    Ok(state)
}

fn receive_sync_profile_at(
    settings: &mut AppSettings,
    state: &mut SyncState,
    default_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = state.profile_directory.as_deref().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "configure a sync profile folder first",
        )
    })?;
    let profile_path = sync_profile_path(directory);
    if !profile_path.is_file() {
        return Err(
            io::Error::new(io::ErrorKind::NotFound, "the sync profile does not exist").into(),
        );
    }
    let profile: SyncProfile = serde_json::from_str(&fs::read_to_string(profile_path)?)?;
    if state.groups.planner
        && let Some(planner) = profile.planner
    {
        settings.planner.events = planner.events;
        settings.planner.tasks = planner.tasks;
        settings.planner.reminders = planner.reminders;
    }
    if state.groups.clocks
        && let Some(mut clocks) = profile.clocks
    {
        if clocks.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the sync profile contains no clocks",
            )
            .into());
        }
        let main_index = clocks.iter().position(|clock| clock.is_main).unwrap_or(0);
        for (index, clock) in clocks.iter_mut().enumerate() {
            clock.is_main = index == main_index;
        }
        settings.clocks_settings = clocks;
    }
    if state.groups.appearance
        && let Some(appearance) = profile.appearance
    {
        settings.planner_appearance = appearance;
    }
    state.last_common_revision = Some(profile.revision);
    state.unresolved_conflict = None;
    save_sync_state_at(default_path, state)
}

fn custom_settings_directory(default_path: &Path) -> Option<PathBuf> {
    let bootstrap_path = location_bootstrap_path(default_path).ok()?;
    let content = fs::read_to_string(bootstrap_path).ok()?;
    let bootstrap: SettingsLocationBootstrap = serde_json::from_str(&content).ok()?;
    bootstrap
        .custom_directory
        .filter(|directory| directory.is_dir())
}

fn active_settings_path(default_path: &Path) -> PathBuf {
    custom_settings_directory(default_path)
        .map(|directory| directory.join(SETTINGS_FILE_NAME))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| default_path.to_path_buf())
}

/// Returns the active settings path, falling back to the platform default when
/// the custom folder or its document is unavailable.
pub fn settings_path() -> io::Result<PathBuf> {
    let default_path = default_settings_path()?;
    Ok(active_settings_path(&default_path))
}

pub fn settings_location() -> io::Result<SettingsLocation> {
    let default_path = default_settings_path()?;
    let path = active_settings_path(&default_path);
    let directory = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "settings path has no parent directory",
        )
    })?;
    Ok(SettingsLocation {
        directory: directory.to_path_buf(),
        uses_default: path == default_path,
    })
}

/// Loads settings, creating the default JSON file on first launch.
pub fn load_or_initialize() -> Result<AppSettings, Box<dyn std::error::Error>> {
    let default_path = default_settings_path()?;
    let path = active_settings_path(&default_path);

    if path != default_path
        && let Ok(content) = fs::read_to_string(&path)
        && let Ok(settings) = serde_json::from_str(&content)
    {
        return Ok(settings);
    }

    load_or_initialize_at(&default_path)
}

fn load_or_initialize_at(path: &Path) -> Result<AppSettings, Box<dyn std::error::Error>> {
    if !path.exists() {
        #[cfg(target_os = "macos")]
        migrate_legacy_settings(
            path,
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
        write_atomically(path, DEFAULT_SETTINGS.as_bytes())?;
    }

    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

/// Persists settings atomically at the configured settings path.
pub fn save(settings: &AppSettings) -> Result<(), Box<dyn std::error::Error>> {
    let content = serde_json::to_string_pretty(settings)?;
    let default_path = default_settings_path()?;
    let path = active_settings_path(&default_path);
    if path != default_path {
        match write_atomically(&path, content.as_bytes()) {
            Ok(()) => return Ok(()),
            Err(_) => {
                write_atomically(&default_path, content.as_bytes())?;
                write_location_bootstrap(&default_path, None)?;
                return Ok(());
            }
        }
    }
    write_atomically(&path, content.as_bytes())?;
    Ok(())
}

/// Moves the complete local settings document into an existing empty folder.
/// The platform-default document is updated first as a recovery copy; only a
/// successful write of the custom document records the new active location.
pub fn move_settings_to_directory(
    settings: &AppSettings,
    directory: &Path,
) -> Result<SettingsLocation, Box<dyn std::error::Error>> {
    let default_path = default_settings_path()?;
    move_settings_to_directory_at(settings, directory, &default_path)
}

fn move_settings_to_directory_at(
    settings: &AppSettings,
    directory: &Path,
    default_path: &Path,
) -> Result<SettingsLocation, Box<dyn std::error::Error>> {
    if !directory.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "choose an existing folder for the settings file",
        )
        .into());
    }
    let target = directory.join(SETTINGS_FILE_NAME);
    if target != default_path && target.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "the selected folder already contains settings.json",
        )
        .into());
    }
    let content = serde_json::to_string_pretty(settings)?;
    write_atomically(default_path, content.as_bytes())?;
    if target != default_path {
        write_atomically(&target, content.as_bytes())?;
        write_location_bootstrap(default_path, Some(directory))?;
    } else {
        write_location_bootstrap(default_path, None)?;
    }
    Ok(SettingsLocation {
        directory: directory.to_path_buf(),
        uses_default: target == default_path,
    })
}

pub fn reset_settings_location(
    settings: &AppSettings,
) -> Result<SettingsLocation, Box<dyn std::error::Error>> {
    let default_path = default_settings_path()?;
    let content = serde_json::to_string_pretty(settings)?;
    write_atomically(&default_path, content.as_bytes())?;
    write_location_bootstrap(&default_path, None)?;
    let directory = default_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "settings path has no parent directory",
        )
    })?;
    Ok(SettingsLocation {
        directory: directory.to_path_buf(),
        uses_default: true,
    })
}

fn write_location_bootstrap(
    default_path: &Path,
    custom_directory: Option<&Path>,
) -> io::Result<()> {
    let bootstrap = SettingsLocationBootstrap {
        custom_directory: custom_directory.map(Path::to_path_buf),
    };
    write_atomically(
        &location_bootstrap_path(default_path)?,
        serde_json::to_vec_pretty(&bootstrap)?.as_slice(),
    )
}

fn write_atomically(path: &Path, content: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "settings path has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;

    let temporary_path = temporary_path(path);
    let write_result = (|| {
        let mut temporary_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        temporary_file.write_all(content)?;
        temporary_file.sync_all()?;
        drop(temporary_file);
        replace_file_atomically(&temporary_path, path)?;
        sync_parent_directory(parent)
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }

    write_result
}

fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let sequence = TEMPORARY_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        sequence
    ))
}

#[cfg(not(windows))]
fn replace_file_atomically(temporary_path: &Path, path: &Path) -> io::Result<()> {
    fs::rename(temporary_path, path)
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> io::Result<()> {
    fs::File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent_directory(_parent: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn replace_file_atomically(temporary_path: &Path, path: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let temporary_path = temporary_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let path = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            temporary_path.as_ptr(),
            path.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };

    if result == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

/// Removes alarm history recorded before the inclusive retention boundary.
pub fn prune_alarm_receipts(settings: &mut AppSettings, now: DateTime<Utc>) {
    let oldest_retained = now - Duration::days(30);
    settings.planner.alarm_receipts.retain(|receipt| {
        DateTime::parse_from_rfc3339(&receipt.recorded_at_utc)
            .map(|recorded| recorded.with_timezone(&Utc) >= oldest_retained)
            .unwrap_or(true)
    });
}

pub fn prune_event_receipts(settings: &mut AppSettings, now: DateTime<Utc>) {
    let oldest_retained = now - Duration::days(30);
    settings.planner.event_receipts.retain(|receipt| {
        receipt.acknowledged_at_utc.is_none()
            || DateTime::parse_from_rfc3339(&receipt.delivered_at_utc)
                .map(|delivered| delivered.with_timezone(&Utc) >= oldest_retained)
                .unwrap_or(true)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn set_08_10_appearance_migration_and_round_trip() {
        let mut settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();
        assert_eq!(settings.planner_appearance, PlannerAppearance::default());
        let partial: PlannerAppearance = serde_json::from_str(r#"{"LightTheme":true}"#).unwrap();
        assert!(partial.light_theme);
        assert!(partial.follow_main_clock);
        assert!(partial.use_accent_color);
        settings.planner_appearance = PlannerAppearance {
            light_theme: true,
            follow_main_clock: false,
            use_accent_color: false,
            custom_accent: "#112233".into(),
            alarm_color: "#223344".into(),
            timer_color: "#334455".into(),
            reminder_color: "#445566".into(),
            event_color: "#556677".into(),
            task_color: "#667788".into(),
        };
        let restored: AppSettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.planner_appearance, settings.planner_appearance);
    }

    #[test]
    fn bundled_defaults_are_valid_and_have_a_main_clock() {
        let settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();

        assert!(!settings.clocks_settings.is_empty());
        assert!(settings.clocks_settings.iter().any(|clock| clock.is_main));
        assert_eq!(settings.corner_radius, 12.0);
        assert!(!settings.soft_clock_style);
        assert_eq!(settings.border_color, "#000000");
        assert_eq!(settings.non_main_dimming, 0.0);
        assert_eq!(settings.alert_sound_duration_seconds, 20);
        assert_eq!(settings.settings_window_height, 672.0);
    }

    #[test]
    fn timer_draft_defaults_and_round_trip() {
        let mut settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();
        assert_eq!(
            (
                settings.timer_draft_days,
                settings.timer_draft_hours,
                settings.timer_draft_minutes,
                settings.timer_draft_seconds
            ),
            (0, 0, 5, 0)
        );
        settings.timer_draft_days = 1;
        settings.timer_draft_hours = 2;
        settings.timer_draft_minutes = 3;
        settings.timer_draft_seconds = 4;
        let restored: AppSettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(
            (
                restored.timer_draft_days,
                restored.timer_draft_hours,
                restored.timer_draft_minutes,
                restored.timer_draft_seconds
            ),
            (1, 2, 3, 4)
        );
    }

    #[test]
    fn alarm_receipts_are_retained_for_thirty_days() {
        let mut settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();
        let alarm_id = PlannerId::new("alarm").unwrap();
        let receipt = |recorded_at_utc: &str| AlarmReceipt {
            alarm_id: alarm_id.clone(),
            occurrence_utc: recorded_at_utc.into(),
            status: AlarmOccurrenceStatus::Delivered,
            recorded_at_utc: recorded_at_utc.into(),
            acknowledged_at_utc: None,
        };
        settings.planner.alarm_receipts = vec![
            receipt("2026-08-05T11:59:59Z"),
            receipt("2026-08-05T12:00:00Z"),
            receipt("2026-09-04T12:00:00Z"),
            receipt("invalid"),
        ];

        prune_alarm_receipts(&mut settings, "2026-09-04T12:00:00Z".parse().unwrap());

        assert_eq!(settings.planner.alarm_receipts.len(), 3);
        assert_eq!(
            settings.planner.alarm_receipts[0].recorded_at_utc,
            "2026-08-05T12:00:00Z"
        );
        assert_eq!(
            settings.planner.alarm_receipts[2].recorded_at_utc,
            "invalid"
        );
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
    fn set_12_moves_settings_and_retains_the_default_recovery_copy() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("oziclock-storage-location-{unique}"));
        let default_path = root.join("default/settings.json");
        let custom_directory = root.join("custom");
        fs::create_dir_all(&custom_directory).unwrap();
        let settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();

        let location =
            move_settings_to_directory_at(&settings, &custom_directory, &default_path).unwrap();

        assert_eq!(location.directory, custom_directory);
        assert!(!location.uses_default);
        assert!(default_path.is_file());
        assert!(custom_directory.join(SETTINGS_FILE_NAME).is_file());
        let bootstrap =
            fs::read_to_string(location_bootstrap_path(&default_path).unwrap()).unwrap();
        assert!(bootstrap.contains("custom"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn set_12_does_not_overwrite_an_existing_custom_document() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("oziclock-storage-existing-{unique}"));
        let default_path = root.join("default/settings.json");
        let custom_directory = root.join("custom");
        fs::create_dir_all(&custom_directory).unwrap();
        let target = custom_directory.join(SETTINGS_FILE_NAME);
        fs::write(&target, "keep this document").unwrap();
        let settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();

        let error =
            move_settings_to_directory_at(&settings, &custom_directory, &default_path).unwrap_err();

        assert_eq!(
            error.to_string(),
            "the selected folder already contains settings.json"
        );
        assert_eq!(fs::read_to_string(target).unwrap(), "keep this document");
        assert!(!default_path.exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn set_11_send_preserves_profile_groups_not_owned_by_this_device() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("oziclock-sync-send-{unique}"));
        let default_path = root.join("default/settings.json");
        let profile_directory = root.join("profile");
        fs::create_dir_all(&profile_directory).unwrap();
        let remote_clocks = vec![ClockSettings {
            label: "Remote".into(),
            time_zone: "UTC".into(),
            color: "#010203".into(),
            is_main: true,
        }];
        let existing_profile = SyncProfile {
            clocks: Some(remote_clocks.clone()),
            ..SyncProfile::default()
        };
        fs::write(
            sync_profile_path(&profile_directory),
            serde_json::to_string(&existing_profile).unwrap(),
        )
        .unwrap();
        let settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();
        let mut state = configure_sync_profile_at(
            &profile_directory,
            SyncGroups {
                planner: true,
                ..SyncGroups::default()
            },
            &default_path,
        )
        .unwrap();

        send_sync_profile_at(&settings, &mut state, &default_path).unwrap();

        let profile: SyncProfile = serde_json::from_str(
            &fs::read_to_string(sync_profile_path(&profile_directory)).unwrap(),
        )
        .unwrap();
        assert_eq!(profile.clocks, Some(remote_clocks));
        assert!(profile.planner.is_some());
        assert_eq!(state.last_common_revision, Some(1));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn set_11_receive_keeps_local_only_settings() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("oziclock-sync-receive-{unique}"));
        let default_path = root.join("default/settings.json");
        let profile_directory = root.join("profile");
        fs::create_dir_all(&profile_directory).unwrap();
        let remote_appearance = PlannerAppearance {
            light_theme: true,
            ..PlannerAppearance::default()
        };
        let profile = SyncProfile {
            revision: 7,
            appearance: Some(remote_appearance.clone()),
            ..SyncProfile::default()
        };
        fs::write(
            sync_profile_path(&profile_directory),
            serde_json::to_string(&profile).unwrap(),
        )
        .unwrap();
        let mut settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();
        settings.show_seconds = true;
        let mut state = configure_sync_profile_at(
            &profile_directory,
            SyncGroups {
                appearance: true,
                ..SyncGroups::default()
            },
            &default_path,
        )
        .unwrap();

        receive_sync_profile_at(&mut settings, &mut state, &default_path).unwrap();

        assert_eq!(settings.planner_appearance, remote_appearance);
        assert!(settings.show_seconds);
        assert_eq!(state.last_common_revision, Some(7));

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
        document.remove("AlertSoundDurationSeconds");
        let settings: AppSettings = serde_json::from_value(legacy).unwrap();

        assert_eq!(settings.corner_radius, 12.0);
        assert!(!settings.soft_clock_style);
        assert_eq!(settings.border_color, "#000000");
        assert_eq!(settings.non_main_dimming, 0.0);
        assert_eq!(settings.alert_sound_duration_seconds, 20);
    }
}
