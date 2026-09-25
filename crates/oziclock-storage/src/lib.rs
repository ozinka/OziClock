//! Versioned JSON settings and local file locations for OziClock.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs,
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
const SYNC_PROFILE_FILE_NAME: &str = "oziclock-sync.json";
static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

mod settings_file;
pub use settings_file::{load_or_initialize, settings_path};

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
    #[serde(default)]
    pub sync: SyncState,
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

fn sync_profile_path(directory: &Path) -> PathBuf {
    directory.join(SYNC_PROFILE_FILE_NAME)
}

/// Saves this device's profile selection inside its local settings document.
pub fn configure_sync_profile(
    settings: &mut AppSettings,
    directory: &Path,
    groups: SyncGroups,
) -> Result<SyncState, Box<dyn std::error::Error>> {
    configure_sync_profile_at(settings, directory, groups, &settings_path()?)
}

/// Summarizes a transfer without modifying the profile or local settings.
pub fn preview_sync_profile(state: &SyncState) -> Result<SyncPreview, Box<dyn std::error::Error>> {
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
    settings: &mut AppSettings,
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
    let mut updated = settings.clone();
    updated.sync = state.clone();
    save_at(&updated, default_path)?;
    *settings = updated;
    Ok(state)
}

/// Writes only this device's selected portable groups to its configured profile.
/// Existing unselected groups remain intact for other devices.
pub fn send_sync_profile(
    settings: &mut AppSettings,
) -> Result<SyncState, Box<dyn std::error::Error>> {
    send_sync_profile_at(settings, &settings_path()?)?;
    Ok(settings.sync.clone())
}

fn send_sync_profile_at(
    settings: &mut AppSettings,
    local_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = settings.sync.clone();
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
    let mut updated = settings.clone();
    updated.sync = state;
    save_at(&updated, local_path)?;
    *settings = updated;
    Ok(())
}

/// Applies only this device's selected portable groups from the configured profile.
/// Local-only scheduling, delivery, timing, and window state are preserved.
pub fn receive_sync_profile(
    settings: &mut AppSettings,
) -> Result<SyncState, Box<dyn std::error::Error>> {
    receive_sync_profile_at(settings, &settings_path()?)?;
    Ok(settings.sync.clone())
}

fn receive_sync_profile_at(
    settings: &mut AppSettings,
    local_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut updated = settings.clone();
    let mut state = settings.sync.clone();
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
        updated.planner.events = planner.events;
        updated.planner.tasks = planner.tasks;
        updated.planner.reminders = planner.reminders;
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
        updated.clocks_settings = clocks;
    }
    if state.groups.appearance
        && let Some(appearance) = profile.appearance
    {
        updated.planner_appearance = appearance;
    }
    state.last_common_revision = Some(profile.revision);
    state.unresolved_conflict = None;
    updated.sync = state;
    save_at(&updated, local_path)?;
    *settings = updated;
    Ok(())
}

/// Persists the single local settings document atomically at its fixed path.
pub fn save(settings: &AppSettings) -> Result<(), Box<dyn std::error::Error>> {
    save_at(settings, &settings_path()?)
}

fn save_at(settings: &AppSettings, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    write_atomically(path, serde_json::to_string_pretty(settings)?.as_bytes())?;
    Ok(())
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
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

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
        let mut settings: AppSettings = serde_json::from_str(DEFAULT_SETTINGS).unwrap();
        configure_sync_profile_at(
            &mut settings,
            &profile_directory,
            SyncGroups {
                planner: true,
                ..SyncGroups::default()
            },
            &default_path,
        )
        .unwrap();

        send_sync_profile_at(&mut settings, &default_path).unwrap();

        let profile: SyncProfile = serde_json::from_str(
            &fs::read_to_string(sync_profile_path(&profile_directory)).unwrap(),
        )
        .unwrap();
        assert_eq!(profile.clocks, Some(remote_clocks));
        assert!(profile.planner.is_some());
        assert_eq!(settings.sync.last_common_revision, Some(1));
        let restored: AppSettings =
            serde_json::from_str(&fs::read_to_string(&default_path).unwrap()).unwrap();
        assert_eq!(restored.sync, settings.sync);
        assert_eq!(
            fs::read_dir(default_path.parent().unwrap())
                .unwrap()
                .count(),
            1
        );
        let profile_json: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(sync_profile_path(&profile_directory)).unwrap(),
        )
        .unwrap();
        assert!(profile_json.get("Sync").is_none());

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
        configure_sync_profile_at(
            &mut settings,
            &profile_directory,
            SyncGroups {
                appearance: true,
                ..SyncGroups::default()
            },
            &default_path,
        )
        .unwrap();

        receive_sync_profile_at(&mut settings, &default_path).unwrap();

        assert_eq!(settings.planner_appearance, remote_appearance);
        assert!(settings.show_seconds);
        assert_eq!(settings.sync.last_common_revision, Some(7));
        let restored: AppSettings =
            serde_json::from_str(&fs::read_to_string(&default_path).unwrap()).unwrap();
        assert_eq!(restored.sync, settings.sync);
        assert_eq!(restored.planner_appearance, remote_appearance);
        assert!(restored.show_seconds);

        // A later receive cannot update memory or revision if the local write fails.
        let blocked = root.join("blocked");
        fs::write(&blocked, "not a directory").unwrap();
        settings.planner_appearance.light_theme = false;
        settings.sync.last_common_revision = Some(6);
        let before = serde_json::to_value(&settings).unwrap();
        assert!(receive_sync_profile_at(&mut settings, &blocked.join("settings.json")).is_err());
        assert_eq!(serde_json::to_value(&settings).unwrap(), before);
        assert!(
            configure_sync_profile_at(
                &mut settings,
                &profile_directory,
                SyncGroups {
                    clocks: true,
                    ..SyncGroups::default()
                },
                &blocked.join("settings.json")
            )
            .is_err()
        );
        assert_eq!(serde_json::to_value(&settings).unwrap(), before);

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
