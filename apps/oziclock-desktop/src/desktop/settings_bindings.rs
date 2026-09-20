use super::colors::parse_color;
use super::{
    AppSettings, AppWindow, AuxiliaryFocusPolicy, ClockListItem, SettingsWindow,
    position_auxiliary_window_near_clock, show_auxiliary_window,
};
use chrono::{DateTime, Offset, Utc};
use chrono_tz::{TZ_VARIANTS, Tz};
use oziclock_app::{ClockCommand, execute_clock_command};
use oziclock_storage::ClockSettings;
use slint::winit_030::winit::dpi::PhysicalSize;
use slint::{ComponentHandle, Model, ModelRc, VecModel, winit_030::WinitWindowAccessor};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TimeZoneOption {
    pub(super) id: String,
    pub(super) display_name: String,
    offset_seconds: i32,
}

pub(super) fn open_settings_window(
    settings_window: &SettingsWindow,
    main_window: &slint::Weak<AppWindow>,
    settings: &AppSettings,
) {
    if let Some(main_window) = main_window.upgrade() {
        main_window.set_modal_open(true);
    }
    let _ = show_auxiliary_window(
        settings_window.window(),
        |_| restore_settings_window_size(settings_window, settings),
        |window| position_auxiliary_window_near_clock(window, main_window),
        AuxiliaryFocusPolicy::Focus,
    );
}

const SETTINGS_WINDOW_WIDTH: f64 = 710.0;
const SETTINGS_WINDOW_HEIGHT: f64 = 672.0;

pub(super) fn restore_settings_window_size(window: &SettingsWindow, settings: &AppSettings) {
    let (width, height) = settings_window_size(settings.settings_window_height);
    window.set_saved_window_width(width as f32);
    window.set_saved_window_height(height as f32);
    let _ = window.window().with_winit_window(|native| {
        native.set_resizable(true);
        let scale_factor = native.scale_factor();
        let _ = native.request_inner_size(PhysicalSize::new(
            (width * scale_factor).round() as u32,
            (height * scale_factor).round() as u32,
        ));
    });
}

pub(super) fn persist_settings_window_size(window: &SettingsWindow, settings: &mut AppSettings) {
    let _ = window.window().with_winit_window(|native| {
        let scale_factor = native.scale_factor();
        let size = native.inner_size();
        let (width, height) = settings_window_size(size.height as f64 / scale_factor);
        settings.settings_window_width = width;
        settings.settings_window_height = height;
        window.set_saved_window_width(width as f32);
        window.set_saved_window_height(height as f32);
    });
}

fn settings_window_size(height: f64) -> (f64, f64) {
    (SETTINGS_WINDOW_WIDTH, height.max(SETTINGS_WINDOW_HEIGHT))
}

pub(super) fn main_clock_index(clocks: &[ClockSettings]) -> usize {
    clocks.iter().position(|clock| clock.is_main).unwrap_or(0)
}

pub(super) fn update_settings_preview(window: &SettingsWindow, settings: &[ClockSettings]) {
    let clocks: Vec<ClockListItem> = settings
        .iter()
        .map(|settings| ClockListItem {
            label: settings.label.clone().into(),
            time_zone: settings.time_zone.clone().into(),
            accent: parse_color(&settings.color),
            is_main: settings.is_main,
        })
        .collect();
    window.set_clocks(ModelRc::new(VecModel::from(clocks)));
}

pub(super) fn select_clock(window: &SettingsWindow, settings: &[ClockSettings], index: i32) {
    if let Some(clock) = settings.get(index.max(0) as usize) {
        window.set_selected_section(1);
        window.set_selected_index(index.max(0));
        window.set_editor_label(clock.label.clone().into());
        window.set_editor_time_zone(clock.time_zone.clone().into());
        window.set_time_zone_search("".into());
        apply_time_zone_filter(window, &time_zone_options(Utc::now()), "");
        window.set_color_picker_open(false);
        window.set_editor_color(clock.color.clone().into());
        window.set_editor_preview_color(parse_color(&clock.color));
        window.set_editor_is_main(clock.is_main);
        window.set_status_message("".into());
    }
}

pub(super) fn time_zone_options(now: DateTime<Utc>) -> Vec<TimeZoneOption> {
    let mut time_zones: Vec<TimeZoneOption> = TZ_VARIANTS
        .iter()
        .map(|time_zone| {
            let offset_seconds = time_zone_offset_seconds(*time_zone, now);
            TimeZoneOption {
                id: time_zone.to_string(),
                display_name: time_zone_display_name(*time_zone, now),
                offset_seconds,
            }
        })
        .collect();
    time_zones.sort_by(|left, right| {
        left.offset_seconds
            .cmp(&right.offset_seconds)
            .then_with(|| left.id.cmp(&right.id))
    });
    time_zones
}

pub(super) fn filter_time_zone_options<'a>(
    time_zones: &'a [TimeZoneOption],
    query: &str,
) -> Vec<&'a TimeZoneOption> {
    let query = query.trim().to_lowercase();
    time_zones
        .iter()
        .filter(|time_zone| {
            query.is_empty()
                || time_zone.id.to_lowercase().contains(&query)
                || time_zone.display_name.to_lowercase().contains(&query)
        })
        .collect()
}

pub(super) fn apply_time_zone_filter(
    window: &SettingsWindow,
    time_zones: &[TimeZoneOption],
    query: &str,
) -> Option<String> {
    let filtered = filter_time_zone_options(time_zones, query);
    let selected_time_zone = window.get_editor_time_zone().to_string();
    let selected_index = filtered_time_zone_index(&filtered, &selected_time_zone);
    window.set_time_zone_ids(ModelRc::new(VecModel::from(
        filtered
            .iter()
            .map(|time_zone| time_zone.id.clone().into())
            .collect::<Vec<_>>(),
    )));
    window.set_time_zones(ModelRc::new(VecModel::from(
        filtered
            .iter()
            .map(|time_zone| time_zone.display_name.clone().into())
            .collect::<Vec<_>>(),
    )));
    window.set_selected_time_zone_index(selected_index);
    window.set_time_zone_search_message(
        if filtered.is_empty() {
            "No matching time zones"
        } else {
            ""
        }
        .into(),
    );
    filtered
        .get(selected_index as usize)
        .filter(|option| option.id != selected_time_zone)
        .map(|option| option.id.clone())
}

fn filtered_time_zone_index(filtered: &[&TimeZoneOption], active_zone: &str) -> i32 {
    filtered
        .iter()
        .position(|option| option.id == active_zone)
        .or_else(|| (!filtered.is_empty()).then_some(0))
        .map(|index| index as i32)
        .unwrap_or(-1)
}

pub(super) fn selected_time_zone_id(window: &SettingsWindow, index: i32) -> Option<String> {
    if index < 0 {
        return None;
    }
    window
        .get_time_zone_ids()
        .row_data(index as usize)
        .map(|time_zone| time_zone.to_string())
}

pub(super) fn time_zone_display_name(time_zone: Tz, now: DateTime<Utc>) -> String {
    let offset_seconds = time_zone_offset_seconds(time_zone, now);
    let sign = if offset_seconds < 0 { '-' } else { '+' };
    let total_minutes = offset_seconds.unsigned_abs() / 60;
    format!(
        "(UTC{sign}{:02}:{:02}) {time_zone}",
        total_minutes / 60,
        total_minutes % 60
    )
}

pub(super) fn time_zone_offset_seconds(time_zone: Tz, now: DateTime<Utc>) -> i32 {
    now.with_timezone(&time_zone)
        .offset()
        .fix()
        .local_minus_utc()
}

pub(super) fn move_selected_clock(
    window: &SettingsWindow,
    settings: &mut Vec<ClockSettings>,
    direction: i32,
) {
    let from = window.get_selected_index();
    let to = from + direction;
    if from < 0 || to < 0 || to >= settings.len() as i32 {
        return;
    }
    execute_clock_command(
        settings,
        ClockCommand::Move {
            from: from as usize,
            to: to as usize,
        },
    );
    update_settings_preview(window, settings);
    select_clock(window, settings, to);
}

pub(super) fn move_clock_to(
    window: &SettingsWindow,
    settings: &mut Vec<ClockSettings>,
    from: i32,
    to: i32,
) {
    if from < 0
        || to < 0
        || from >= settings.len() as i32
        || to >= settings.len() as i32
        || from == to
    {
        return;
    }
    execute_clock_command(
        settings,
        ClockCommand::Move {
            from: from as usize,
            to: to as usize,
        },
    );
    update_settings_preview(window, settings);
    select_clock(window, settings, to);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn options() -> Vec<TimeZoneOption> {
        time_zone_options(Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap())
    }

    #[test]
    fn clk_08b_filter_selects_japan_immediately_from_utc() {
        let options = options();
        let filtered = filter_time_zone_options(&options, "japa");
        let index = filtered_time_zone_index(&filtered, "UTC");
        assert_eq!(index, 0);
        assert_eq!(filtered[index as usize].id, "Japan");
        assert_eq!(filtered[index as usize].offset_seconds, 9 * 3600);
    }

    #[test]
    fn clk_08b_multiple_matches_keep_active_zone_or_select_first() {
        let options = options();
        let filtered = filter_time_zone_options(&options, "america/");
        let index = filtered_time_zone_index(&filtered, "America/New_York");
        assert_eq!(filtered[index as usize].id, "America/New_York");
        assert_eq!(filtered_time_zone_index(&filtered, "UTC"), 0);
        let empty = filter_time_zone_options(&options, "no-such-zone");
        assert_eq!(filtered_time_zone_index(&empty, "America/New_York"), -1);
    }

    #[test]
    fn clk_08d_full_list_preserves_each_clocks_configured_zone() {
        let options = options();
        let restored = filter_time_zone_options(&options, "");
        assert_eq!(restored.len(), options.len());
        for zone in ["UTC", "Japan", "America/New_York"] {
            let index = filtered_time_zone_index(&restored, zone);
            assert_eq!(restored[index as usize].id, zone);
        }
    }

    #[test]
    fn empty_query_returns_every_time_zone_in_original_order() {
        let options = options();
        let filtered = filter_time_zone_options(&options, "  ");

        assert_eq!(filtered.len(), options.len());
        assert!(
            filtered
                .iter()
                .zip(&options)
                .all(|(filtered, original)| filtered.id == original.id)
        );
    }

    #[test]
    fn search_is_case_insensitive_and_matches_iana_identifier() {
        let options = options();
        let filtered = filter_time_zone_options(&options, "new_YORK");

        assert!(
            filtered
                .iter()
                .any(|option| option.id == "America/New_York")
        );
    }

    #[test]
    fn search_matches_visible_utc_offset() {
        let options = options();
        let filtered = filter_time_zone_options(&options, "utc+05:45");

        assert!(filtered.iter().any(|option| option.id == "Asia/Kathmandu"));
    }

    #[test]
    fn filtered_results_preserve_offset_then_identifier_order() {
        let options = options();
        let filtered = filter_time_zone_options(&options, "america/");

        assert!(filtered.windows(2).all(|pair| {
            pair[0].offset_seconds < pair[1].offset_seconds
                || pair[0].offset_seconds == pair[1].offset_seconds && pair[0].id <= pair[1].id
        }));
    }

    #[test]
    fn unmatched_query_returns_no_results() {
        assert!(filter_time_zone_options(&options(), "not-a-real-time-zone").is_empty());
    }

    #[test]
    fn ui_09_settings_window_width_is_fixed_and_height_has_a_safe_minimum() {
        assert_eq!(settings_window_size(632.0), (710.0, 672.0));
        assert_eq!(settings_window_size(860.0), (710.0, 860.0));
    }
}
