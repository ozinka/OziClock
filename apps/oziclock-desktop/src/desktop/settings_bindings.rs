use super::colors::parse_color;
use super::{
    AppSettings, AppWindow, ClockListItem, SettingsWindow, focus_auxiliary_window,
    hide_auxiliary_window_from_taskbar, position_auxiliary_window_near_clock,
};
use chrono::{DateTime, Offset, Utc};
use chrono_tz::{TZ_VARIANTS, Tz};
use oziclock_app::{ClockCommand, execute_clock_command};
use oziclock_storage::ClockSettings;
use slint::winit_030::winit::dpi::PhysicalSize;
use slint::{ComponentHandle, ModelRc, VecModel, winit_030::WinitWindowAccessor};

pub(super) fn open_settings_window(
    settings_window: &SettingsWindow,
    main_window: &slint::Weak<AppWindow>,
    settings: &AppSettings,
) {
    let _ = settings_window.show();
    restore_settings_window_size(settings_window, settings);
    hide_auxiliary_window_from_taskbar(settings_window.window());
    position_auxiliary_window_near_clock(settings_window.window(), main_window);
    focus_auxiliary_window(settings_window.window());
}

pub(super) fn restore_settings_window_size(window: &SettingsWindow, settings: &AppSettings) {
    let width = settings.settings_window_width.max(760.0);
    let height = settings.settings_window_height.max(620.0);
    window.set_saved_window_width(width as f32);
    window.set_saved_window_height(height as f32);
    let _ = window.window().with_winit_window(|native| {
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
        settings.settings_window_width = size.width as f64 / scale_factor;
        settings.settings_window_height = size.height as f64 / scale_factor;
        window.set_saved_window_width(settings.settings_window_width as f32);
        window.set_saved_window_height(settings.settings_window_height as f32);
    });
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
        window.set_selected_time_zone_index(time_zone_index(&clock.time_zone));
        window.set_editor_color(clock.color.clone().into());
        window.set_editor_preview_color(parse_color(&clock.color));
        window.set_editor_is_main(clock.is_main);
        window.set_status_message("".into());
    }
}

pub(super) fn time_zone_index(time_zone: &str) -> i32 {
    let now = Utc::now();
    let mut time_zones: Vec<(String, i32)> = TZ_VARIANTS
        .iter()
        .map(|candidate| {
            (
                candidate.to_string(),
                time_zone_offset_seconds(*candidate, now),
            )
        })
        .collect();
    time_zones.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
    time_zones
        .iter()
        .position(|(candidate, _)| candidate == time_zone)
        .map(|index| index as i32)
        .unwrap_or(-1)
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
