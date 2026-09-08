slint::include_modules!();

mod alert_sound;
mod calendar_bindings;
mod clock_refresh;
mod colors;
mod delivery_feedback;
mod launch_at_login;
mod planner_alarm_bindings;
mod planner_inputs;
mod planner_models;
mod planner_timer_bindings;
mod settings_bindings;
#[cfg(target_os = "windows")]
mod tray;
mod window_drag;
#[cfg(target_os = "macos")]
mod window_opacity;

use calendar_bindings::{
    CalendarState, CalendarView, initial_week_scroll_y, normalize_week_scroll,
    refresh_calendar_window,
};
use clock_refresh::schedule_clock_refresh;
use colors::{color_to_hsv, hsv_color, hsv_hex, parse_color};
use planner_inputs::{
    adjust_timer_part, default_alarm_time, event_alert_label, event_recurrence_label,
    mask_clock_time, mask_timer_part, normalize_alarm_time, parse_alert_with_custom,
    parse_event_recurrence, task_alert_label,
};
use planner_models::{
    completed_task_count, format_stopwatch, open_task_count, plan_counts_for_date, plan_event_rows,
    plan_reminder_rows, plan_task_rows, planner_completed_task_rows, planner_reminder_rows,
    planner_task_rows, stopwatch_lap_rows,
};
use settings_bindings::{
    apply_time_zone_filter, main_clock_index, move_clock_to, move_selected_clock,
    open_settings_window, persist_settings_window_size, select_clock, selected_time_zone_id,
    time_zone_options, update_settings_preview,
};
#[cfg(target_os = "windows")]
use tray::create_system_tray;
use window_drag::configure_main_window_drag;

use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
    time::{Duration, Instant},
};

use chrono::{
    DateTime, Datelike, LocalResult, Months, NaiveDate, NaiveDateTime, Offset, TimeZone, Timelike,
    Utc,
};
use chrono_tz::Tz;
use oziclock_app::calendar::CalendarDate;
use oziclock_app::planner::{PlannerCommand, execute_planner_command};
use oziclock_app::{ClockCommand, execute_clock_command};
use oziclock_storage::{
    AlertRule, AppSettings, ClockSettings, Event, EventReceipt, EventTime, PlannerId, Reminder,
    ReminderRecurrence, ReminderSchedule, Stopwatch, StopwatchState, Task, TaskStatus,
};
use slint::winit_030::EventResult;
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::dpi::{LogicalPosition, PhysicalPosition, PhysicalSize};
#[cfg(target_os = "macos")]
use slint::winit_030::winit::platform::macos::WindowAttributesExtMacOS;
#[cfg(target_os = "windows")]
use slint::winit_030::winit::platform::windows::{MonitorHandleExtWindows, WindowExtWindows};
use slint::winit_030::winit::{
    event::{ElementState, WindowEvent},
    keyboard::{Key, NamedKey},
};
use slint::{Model, ModelRc, Timer, TimerMode, VecModel};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO};

pub(crate) fn run() -> Result<(), slint::PlatformError> {
    let alert_sound = alert_sound::AlertSound::new();
    let mut settings = oziclock_storage::load_or_initialize().map_err(|error| {
        slint::PlatformError::Other(format!("could not load OziClock settings: {error}"))
    })?;
    let clock_scale_percent = normalize_clock_scale_percent((settings.clock_scale * 100.0) as f32);
    settings.clock_scale = f64::from(clock_scale_percent / 100.0);
    settings.border_color =
        normalize_border_color(&settings.border_color).unwrap_or_else(|| "#000000".to_owned());
    settings.non_main_dimming = settings.non_main_dimming.clamp(0.0, 80.0);
    settings.alert_sound_duration_seconds =
        normalize_alert_sound_duration_seconds(settings.alert_sound_duration_seconds);
    let initial_main_window_position =
        LogicalPosition::new(settings.main_wnd_left, settings.main_wnd_top);
    let is_first_native_window = Rc::new(Cell::new(true));
    let first_native_window_for_hook = is_first_native_window.clone();
    slint::BackendSelector::new()
        .with_winit_window_attributes_hook(move |attributes| {
            if first_native_window_for_hook.replace(false) {
                #[cfg(target_os = "macos")]
                let attributes = attributes.with_has_shadow(true);
                attributes.with_position(initial_main_window_position)
            } else {
                attributes
            }
        })
        .select()?;
    let window = AppWindow::new()?;
    #[cfg(target_os = "macos")]
    window_opacity::configure(&window);
    window.set_product_name(oziclock_app::application_name().into());
    window.set_top_most(settings.top_most);
    window.set_inactive_opacity(settings.opacity.clamp(0.02, 1.0) as f32);
    window.set_show_seconds(settings.show_seconds);
    window.set_show_rulers(settings.show_rulers);
    window.set_compact_mode(settings.compact_mode);
    window.set_compact_progress(if settings.compact_mode { 1.0 } else { 0.0 });
    window.set_corner_radius(settings.corner_radius.clamp(0.0, 15.5) as f32);
    window.set_soft_clock_style(settings.soft_clock_style);
    window.set_border_color(parse_color(&settings.border_color));
    window.set_non_main_dimming(settings.non_main_dimming as f32);
    apply_clock_scale(&window, settings.clock_scale.clamp(0.8, 1.5) as f32);
    update_clock_tiles(&window, &settings.clocks_settings, settings.show_seconds);

    let settings_window = SettingsWindow::new()?;
    let settings_for_keyboard = settings_window.as_weak();
    settings_window
        .window()
        .on_winit_window_event(move |_, event| {
            if is_escape_key(event) || matches!(event, WindowEvent::CloseRequested) {
                if let Some(settings_window) = settings_for_keyboard.upgrade() {
                    settings_window.invoke_request_close();
                }
                return EventResult::PreventDefault;
            }
            if is_enter_key(event) {
                if let Some(settings_window) = settings_for_keyboard.upgrade() {
                    if settings_window.get_time_zone_search_focused()
                        || settings_window.get_time_zone_picker_focused()
                    {
                        return EventResult::Propagate;
                    }
                    settings_window.invoke_request_save();
                }
                return EventResult::PreventDefault;
            }
            EventResult::Propagate
        });
    let now = Utc::now();
    let time_zone_options = Rc::new(time_zone_options(now));
    apply_time_zone_filter(&settings_window, &time_zone_options, "");
    settings_window.set_show_seconds(settings.show_seconds);
    settings_window.set_top_most(settings.top_most);
    settings_window.set_show_in_task_bar(settings.show_in_task_bar);
    settings_window.set_launch_at_login(settings.launch_at_login);
    settings_window.set_compact_mode(settings.compact_mode);
    settings_window.set_clock_scale_percent(clock_scale_percent);
    settings_window.set_corner_radius(settings.corner_radius.clamp(0.0, 15.5) as f32);
    settings_window.set_soft_clock_style(settings.soft_clock_style);
    settings_window.set_border_color_value(settings.border_color.clone().into());
    settings_window.set_border_preview_color(parse_color(&settings.border_color));
    settings_window.set_non_main_dimming(settings.non_main_dimming as f32);
    settings_window.set_calendar_light_theme(settings.calendar_light_theme);
    settings_window.set_calendar_monday_first(settings.calendar_monday_first);
    settings_window.set_calendar_hour_range(i32::from(settings.calendar_hour_range.min(2)));
    settings_window.set_alert_sound_duration_seconds(i32::from(
        normalize_alert_sound_duration_seconds(settings.alert_sound_duration_seconds),
    ));
    settings_window.set_opacity_percent((settings.opacity.clamp(0.02, 1.0) * 100.0) as f32);
    update_settings_preview(&settings_window, &settings.clocks_settings);
    select_clock(&settings_window, &settings.clocks_settings, 0);
    settings_window.set_selected_section(0);
    initialize_ruler_content(&window, &settings);
    let shared_settings = Rc::new(RefCell::new(settings));
    if shared_settings.borrow().launch_at_login {
        let _ = launch_at_login::set_enabled(true);
    }
    let calendar_window = CalendarWindow::new()?;
    let planner_window = PlannerWindow::new()?;
    let alarm_attention_window = AlarmAttentionWindow::new()?;
    let timer_attention_window = TimerAttentionWindow::new()?;
    let reminder_attention_window = ReminderAttentionWindow::new()?;
    let event_attention_window = EventAttentionWindow::new()?;
    let task_attention_window = TaskAttentionWindow::new()?;
    let planner_accent = calendar_accent(&shared_settings.borrow()).brighter(0.4);
    planner_window.set_accent(planner_accent);
    alarm_attention_window.set_accent(planner_accent);
    timer_attention_window.set_accent(planner_accent);
    reminder_attention_window.set_accent(planner_accent);
    event_attention_window.set_accent(planner_accent);
    task_attention_window.set_accent(planner_accent);
    planner_window
        .set_corner_radius(shared_settings.borrow().corner_radius.clamp(0.0, 15.5) as f32);
    planner_window.set_task_count(open_task_count(&shared_settings.borrow()));
    planner_window.set_completed_task_count(completed_task_count(&shared_settings.borrow()));
    planner_window
        .set_timer_days_draft(shared_settings.borrow().timer_draft_days.to_string().into());
    planner_window.set_timer_hours_draft(
        shared_settings
            .borrow()
            .timer_draft_hours
            .to_string()
            .into(),
    );
    planner_window.set_timer_minutes_draft(
        shared_settings
            .borrow()
            .timer_draft_minutes
            .to_string()
            .into(),
    );
    planner_window.set_timer_seconds_draft(
        shared_settings
            .borrow()
            .timer_draft_seconds
            .to_string()
            .into(),
    );
    let (reminder_date, reminder_time) = default_reminder_datetime(&shared_settings.borrow());
    planner_window.set_task_date_draft(reminder_date.clone().into());
    planner_window.set_task_time_draft(reminder_time.clone().into());
    planner_window.set_reminder_date_draft(reminder_date.into());
    planner_window.set_reminder_hours_draft(reminder_time[..2].into());
    planner_window.set_reminder_minutes_draft(reminder_time[3..].into());
    let initial_plan_minute = calendar_local_now(&shared_settings.borrow())
        .time()
        .num_seconds_from_midnight()
        / 60;
    planner_window.set_plan_scroll_y(-(initial_plan_minute.saturating_sub(300) as f32));
    let planner_for_alarm_entry = planner_window.as_weak();
    let settings_for_alarm_entry = shared_settings.clone();
    planner_window.on_request_alarm_section_enter(move || {
        let Some(planner) = planner_for_alarm_entry.upgrade() else {
            return;
        };
        if !planner.get_editing_alarm().is_empty() {
            return;
        }
        let default = default_alarm_time(calendar_local_now(&settings_for_alarm_entry.borrow()));
        planner.set_alarm_hours_draft(default[..2].into());
        planner.set_alarm_minutes_draft(default[3..].into());
    });
    let planner_for_reminder_entry = planner_window.as_weak();
    let settings_for_reminder_entry = shared_settings.clone();
    planner_window.on_request_reminder_section_enter(move || {
        let Some(planner) = planner_for_reminder_entry.upgrade() else {
            return;
        };
        if !planner.get_editing_reminder().is_empty() {
            return;
        }
        let (date, time) = default_reminder_datetime(&settings_for_reminder_entry.borrow());
        planner.set_reminder_date_draft(date.into());
        planner.set_reminder_hours_draft(time[..2].into());
        planner.set_reminder_minutes_draft(time[3..].into());
    });
    let reminder_calendar_model = Rc::new(VecModel::from(Vec::<ReminderCalendarDayData>::new()));
    planner_window.set_reminder_calendar_days(ModelRc::from(reminder_calendar_model.clone()));
    let reminder_calendar_anchor = Rc::new(RefCell::new(Utc::now().date_naive()));
    let planner_for_open_reminder_calendar = planner_window.as_weak();
    let settings_for_open_reminder_calendar = shared_settings.clone();
    let model_for_open_reminder_calendar = reminder_calendar_model.clone();
    let anchor_for_open_reminder_calendar = reminder_calendar_anchor.clone();
    planner_window.on_request_open_reminder_calendar(move || {
        let Some(planner) = planner_for_open_reminder_calendar.upgrade() else {
            return;
        };
        let today = calendar_local_now(&settings_for_open_reminder_calendar.borrow()).date();
        let selected_text = if planner.get_editor_modal() == 5 {
            planner.get_task_date_draft()
        } else if planner.get_editor_modal() == 6 {
            if planner.get_event_calendar_target() == 1 {
                planner.get_event_end_date_draft()
            } else {
                planner.get_event_date_draft()
            }
        } else {
            planner.get_reminder_date_draft()
        };
        let selected = NaiveDate::parse_from_str(&selected_text, "%Y-%m-%d").ok();
        let anchor = selected
            .unwrap_or(today)
            .with_day(1)
            .expect("the first day exists in every month");
        *anchor_for_open_reminder_calendar.borrow_mut() = anchor;
        refresh_reminder_calendar(
            &planner,
            &model_for_open_reminder_calendar,
            anchor,
            selected,
            today,
        );
        planner.set_reminder_calendar_visible(true);
    });
    let planner_for_navigate_reminder_calendar = planner_window.as_weak();
    let settings_for_navigate_reminder_calendar = shared_settings.clone();
    let model_for_navigate_reminder_calendar = reminder_calendar_model.clone();
    let anchor_for_navigate_reminder_calendar = reminder_calendar_anchor.clone();
    planner_window.on_request_navigate_reminder_calendar(move |direction| {
        let Some(planner) = planner_for_navigate_reminder_calendar.upgrade() else {
            return;
        };
        let current = *anchor_for_navigate_reminder_calendar.borrow();
        let shifted = if direction < 0 {
            current.checked_sub_months(Months::new(1))
        } else {
            current.checked_add_months(Months::new(1))
        };
        let Some(anchor) = shifted else {
            return;
        };
        *anchor_for_navigate_reminder_calendar.borrow_mut() = anchor;
        let today = calendar_local_now(&settings_for_navigate_reminder_calendar.borrow()).date();
        let selected_text = if planner.get_editor_modal() == 5 {
            planner.get_task_date_draft()
        } else if planner.get_editor_modal() == 6 {
            if planner.get_event_calendar_target() == 1 {
                planner.get_event_end_date_draft()
            } else {
                planner.get_event_date_draft()
            }
        } else {
            planner.get_reminder_date_draft()
        };
        let selected = NaiveDate::parse_from_str(&selected_text, "%Y-%m-%d").ok();
        refresh_reminder_calendar(
            &planner,
            &model_for_navigate_reminder_calendar,
            anchor,
            selected,
            today,
        );
    });
    let planner_for_select_reminder_date = planner_window.as_weak();
    planner_window.on_request_select_reminder_date(move |date| {
        if let Some(planner) = planner_for_select_reminder_date.upgrade() {
            if planner.get_editor_modal() == 5 {
                planner.set_task_date_draft(date);
                planner.set_task_error("".into());
            } else if planner.get_editor_modal() == 6 {
                if planner.get_event_calendar_target() == 1 {
                    planner.set_event_end_date_draft(date);
                } else {
                    planner.set_event_date_draft(date);
                }
                planner.set_event_error("".into());
            } else {
                planner.set_reminder_date_draft(date);
                planner.set_reminder_error("".into());
            }
            planner.set_reminder_calendar_visible(false);
        }
    });
    let plan_day_model = Rc::new(VecModel::from(Vec::<PlanWeekDayData>::new()));
    let plan_task_model = Rc::new(VecModel::from(Vec::<PlanTaskMarkerData>::new()));
    let plan_month_model = Rc::new(VecModel::from(Vec::<PlanMonthDayData>::new()));
    let plan_year_model = Rc::new(VecModel::from(Vec::<PlanYearMonthData>::new()));
    let plan_reminder_model = Rc::new(VecModel::from(Vec::<PlanReminderMarkerData>::new()));
    let plan_event_model = Rc::new(VecModel::from(Vec::<PlanEventMarkerData>::new()));
    let plan_all_day_event_model = Rc::new(VecModel::from(Vec::<PlanAllDayEventData>::new()));
    planner_window.set_plan_week_days(ModelRc::from(plan_day_model.clone()));
    planner_window.set_plan_tasks(ModelRc::from(plan_task_model.clone()));
    planner_window.set_plan_month_days(ModelRc::from(plan_month_model.clone()));
    planner_window.set_plan_year_months(ModelRc::from(plan_year_model.clone()));
    planner_window.set_plan_reminders(ModelRc::from(plan_reminder_model.clone()));
    planner_window.set_plan_events(ModelRc::from(plan_event_model.clone()));
    planner_window.set_plan_all_day_events(ModelRc::from(plan_all_day_event_model.clone()));
    let plan_week_start = Rc::new(RefCell::new(current_week_start(&shared_settings.borrow())));
    refresh_plan_week(
        &planner_window,
        &plan_day_model,
        &plan_task_model,
        &plan_reminder_model,
        (&plan_event_model, &plan_all_day_event_model),
        &shared_settings.borrow(),
        *plan_week_start.borrow(),
    );
    refresh_plan_overview(
        &planner_window,
        &plan_month_model,
        &plan_year_model,
        &shared_settings.borrow(),
        calendar_local_now(&shared_settings.borrow()).date(),
    );
    let planner_for_plan_week = planner_window.as_weak();
    let settings_for_plan_week = shared_settings.clone();
    let days_for_plan_week = plan_day_model.clone();
    let reminders_for_plan_week = plan_reminder_model.clone();
    let tasks_for_plan_week = plan_task_model.clone();
    let month_for_plan = plan_month_model.clone();
    let year_for_plan = plan_year_model.clone();
    let events_for_plan_week = plan_event_model.clone();
    let all_day_events_for_plan_week = plan_all_day_event_model.clone();
    let start_for_plan_week = plan_week_start.clone();
    planner_window.on_request_plan_week(move |direction| {
        let Some(planner) = planner_for_plan_week.upgrade() else {
            return;
        };
        let view = planner.get_selected_view();
        let current = *start_for_plan_week.borrow();
        let today = calendar_local_now(&settings_for_plan_week.borrow()).date();
        let start = if view == 0 {
            if direction == 0 {
                current_week_start(&settings_for_plan_week.borrow())
            } else {
                current + chrono::Duration::days(i64::from(direction.signum()) * 7)
            }
        } else if view == 1 {
            let base = if direction == 0 {
                today
            } else if direction >= 100 {
                today.with_month(direction as u32 - 100).unwrap_or(today)
            } else {
                current
            };
            let shifted = if (-1..=1).contains(&direction) && direction != 0 {
                if direction < 0 {
                    base.checked_sub_months(Months::new(1))
                } else {
                    base.checked_add_months(Months::new(1))
                }
                .unwrap_or(base)
            } else {
                base
            };
            shifted.with_day(1).unwrap_or(shifted)
        } else {
            let base = if direction == 0 { today } else { current };
            let shifted = if direction < 0 {
                base.checked_sub_months(Months::new(12))
            } else if direction > 0 {
                base.checked_add_months(Months::new(12))
            } else {
                Some(base)
            }
            .unwrap_or(base);
            shifted
                .with_month(1)
                .and_then(|date| date.with_day(1))
                .unwrap_or(shifted)
        };
        *start_for_plan_week.borrow_mut() = start;
        planner.set_selected_plan_date("".into());
        planner.set_selected_plan_hour(-1);
        planner.set_selected_plan_reminder("".into());
        planner.set_selected_plan_event("".into());
        planner.set_selected_plan_task("".into());
        planner.set_selected_plan_title("".into());
        planner.set_selected_plan_time("".into());
        if view == 0 {
            refresh_plan_week(
                &planner,
                &days_for_plan_week,
                &tasks_for_plan_week,
                &reminders_for_plan_week,
                (&events_for_plan_week, &all_day_events_for_plan_week),
                &settings_for_plan_week.borrow(),
                start,
            );
        } else {
            refresh_plan_overview(
                &planner,
                &month_for_plan,
                &year_for_plan,
                &settings_for_plan_week.borrow(),
                start,
            );
        }
    });
    schedule_plan_refresh(
        Rc::new(Timer::default()),
        planner_window.as_weak(),
        PlanRefreshModels {
            days: plan_day_model.clone(),
            tasks: plan_task_model.clone(),
            month_days: plan_month_model.clone(),
            year_months: plan_year_model.clone(),
            reminders: plan_reminder_model.clone(),
            events: plan_event_model.clone(),
            all_day_events: plan_all_day_event_model.clone(),
        },
        shared_settings.clone(),
        plan_week_start.clone(),
    );
    let planner_for_new_event = planner_window.as_weak();
    planner_window.on_request_new_event(move |date, hour| {
        let Some(planner) = planner_for_new_event.upgrade() else {
            return;
        };
        let start_hour = hour.clamp(0, 23);
        let end_minutes = (start_hour * 60 + 60).min(23 * 60 + 59);
        planner.set_editing_event("".into());
        planner.set_event_title_draft("Event".into());
        planner.set_event_date_draft(date.clone());
        planner.set_event_end_date_draft(date);
        planner.set_event_start_draft(format!("{start_hour:02}:00").into());
        planner
            .set_event_end_draft(format!("{:02}:{:02}", end_minutes / 60, end_minutes % 60).into());
        planner.set_event_all_day(false);
        planner.set_event_recurrence("Does not repeat".into());
        planner.set_event_alert("No alert".into());
        planner.set_event_error("".into());
    });
    let planner_for_adjust_event_time = planner_window.as_weak();
    planner_window.on_request_adjust_event_time(move |target, direction| {
        let Some(planner) = planner_for_adjust_event_time.upgrade() else {
            return;
        };
        let value = if target == 0 {
            planner.get_event_start_draft()
        } else {
            planner.get_event_end_draft()
        };
        let Some(normalized) = normalize_alarm_time(&value) else {
            planner.set_event_error("Enter time as HH:MM.".into());
            return;
        };
        let hours = normalized[..2].parse::<i32>().unwrap_or_default();
        let minutes = normalized[3..].parse::<i32>().unwrap_or_default();
        let adjusted = (hours * 60 + minutes + direction.signum() * 15).clamp(0, 1_439);
        let adjusted = format!("{:02}:{:02}", adjusted / 60, adjusted % 60).into();
        if target == 0 {
            planner.set_event_start_draft(adjusted);
        } else {
            planner.set_event_end_draft(adjusted);
        }
        planner.set_event_error("".into());
    });
    let planner_for_edit_event_time = planner_window.as_weak();
    planner_window.on_request_edit_event_time(move |target, text| {
        let Some(planner) = planner_for_edit_event_time.upgrade() else {
            return;
        };
        let masked = mask_clock_time(&text).into();
        if target == 0 {
            planner.set_event_start_draft(masked);
            planner.set_event_error("".into());
        } else if target == 1 {
            planner.set_event_end_draft(masked);
            planner.set_event_error("".into());
        } else {
            planner.set_task_time_draft(masked);
            planner.set_task_error("".into());
        }
    });
    let planner_for_edit_event = planner_window.as_weak();
    let settings_for_edit_event = shared_settings.clone();
    planner_window.on_request_edit_event(move |event_id| {
        let settings = settings_for_edit_event.borrow();
        let Some(event) = settings
            .planner
            .events
            .iter()
            .find(|event| event.id.to_string() == event_id.as_str())
        else {
            return;
        };
        let Some(planner) = planner_for_edit_event.upgrade() else {
            return;
        };
        let zone = main_time_zone(&settings)
            .parse::<Tz>()
            .unwrap_or(chrono_tz::UTC);
        planner.set_editing_event(event_id);
        planner.set_event_title_draft(event.title.clone().into());
        planner.set_event_recurrence(event_recurrence_label(event.recurrence).into());
        planner.set_event_alert(event_alert_label(event.alerts.first()).into());
        if let Some(alert) = event
            .alerts
            .first()
            .filter(|alert| !matches!(alert.offset_minutes, 0 | 5 | 15 | 30 | 60))
        {
            planner.set_event_custom_alert_minutes(alert.offset_minutes.to_string().into());
        }
        planner.set_event_error("".into());
        match &event.time {
            EventTime::Timed {
                start_utc, end_utc, ..
            } => {
                let (Ok(start), Ok(end)) = (
                    DateTime::parse_from_rfc3339(start_utc),
                    DateTime::parse_from_rfc3339(end_utc),
                ) else {
                    planner.set_event_error("This event has an invalid saved time.".into());
                    return;
                };
                let start = start.with_timezone(&zone);
                let end = end.with_timezone(&zone);
                planner.set_event_date_draft(start.format("%Y-%m-%d").to_string().into());
                planner.set_event_end_date_draft(end.format("%Y-%m-%d").to_string().into());
                planner.set_event_start_draft(start.format("%H:%M").to_string().into());
                planner.set_event_end_draft(end.format("%H:%M").to_string().into());
                planner.set_event_all_day(false);
            }
            EventTime::AllDay {
                start_date,
                end_date,
            } => {
                planner.set_event_date_draft(start_date.clone().into());
                planner.set_event_end_date_draft(end_date.clone().into());
                planner.set_event_all_day(true);
            }
        }
    });
    let event_attention_queue = Rc::new(RefCell::new(pending_event_attention_items(
        &shared_settings.borrow(),
    )));
    let planner_for_save_event = planner_window.as_weak();
    let settings_for_save_event = shared_settings.clone();
    let event_model_for_save_event = plan_event_model.clone();
    let all_day_model_for_save_event = plan_all_day_event_model.clone();
    let start_for_save_event = plan_week_start.clone();
    let queue_for_save_event = event_attention_queue.clone();
    let attention_for_save_event = event_attention_window.as_weak();
    let owner_for_save_event = window.as_weak();
    planner_window.on_request_save_event(
        move |title, date, end_date, start, end, all_day, recurrence, alert| {
            let Some(planner) = planner_for_save_event.upgrade() else {
                return;
            };
            planner.set_event_error("".into());
            let title = title.trim();
            if title.is_empty() {
                planner.set_event_error("Enter an event title.".into());
                return;
            }
            if NaiveDate::parse_from_str(&date, "%Y-%m-%d").is_err() {
                planner.set_event_error("Enter a valid date.".into());
                return;
            }
            let mut settings = settings_for_save_event.borrow_mut();
            let zone = main_time_zone(&settings);
            let time = if all_day {
                let Ok(start_date) = NaiveDate::parse_from_str(&date, "%Y-%m-%d") else {
                    planner.set_event_error("Enter a valid start date.".into());
                    return;
                };
                let Ok(parsed_end_date) = NaiveDate::parse_from_str(&end_date, "%Y-%m-%d") else {
                    planner.set_event_error("Enter a valid end date.".into());
                    return;
                };
                if parsed_end_date < start_date {
                    planner.set_event_error("End date must not be before start date.".into());
                    return;
                }
                EventTime::AllDay {
                    start_date: date.to_string(),
                    end_date: end_date.to_string(),
                }
            } else {
                let Some(start_utc) =
                    oziclock_app::reminder_time::resolve_local(&date, &start, &zone)
                else {
                    planner.set_event_error("Enter a valid start time.".into());
                    return;
                };
                let Some(end_utc) = oziclock_app::reminder_time::resolve_local(&date, &end, &zone)
                else {
                    planner.set_event_error("Enter a valid end time.".into());
                    return;
                };
                if end_utc <= start_utc {
                    planner.set_event_error("End time must be later than start time.".into());
                    return;
                }
                EventTime::Timed {
                    start_utc: start_utc.to_rfc3339(),
                    end_utc: end_utc.to_rfc3339(),
                    source_time_zone: zone,
                }
            };
            let mut updated = settings.clone();
            let Some(recurrence) = parse_event_recurrence(&recurrence) else {
                planner.set_event_error("Select a valid repeat option.".into());
                return;
            };
            let Some(alert_offset) = parse_alert_with_custom(
                &alert,
                &planner.get_event_custom_alert_minutes(),
                "At start",
            ) else {
                planner.set_event_error("Select a valid alert option.".into());
                return;
            };
            let alerts = alert_offset
                .map(|offset_minutes| AlertRule {
                    id: PlannerId::new(format!(
                        "event-alert-{}",
                        Utc::now().timestamp_nanos_opt().unwrap_or_default()
                    ))
                    .expect("generated alert id is valid"),
                    offset_minutes,
                    play_sound: true,
                })
                .into_iter()
                .collect::<Vec<_>>();
            let editing = planner.get_editing_event();
            let edited_id = PlannerId::new(editing.to_string());
            let changed = if let Some(id) = edited_id.clone() {
                execute_planner_command(
                    &mut updated.planner,
                    PlannerCommand::UpdateEvent {
                        id,
                        title: title.to_owned(),
                        time,
                        recurrence,
                        alerts,
                    },
                )
            } else {
                execute_planner_command(
                    &mut updated.planner,
                    PlannerCommand::AddEvent {
                        event: Event {
                            id: PlannerId::new(format!(
                                "event-{}",
                                Utc::now().timestamp_nanos_opt().unwrap_or_default()
                            ))
                            .expect("generated event id is valid"),
                            title: title.to_owned(),
                            time,
                            location: None,
                            notes: None,
                            link: None,
                            color: None,
                            alerts,
                            recurrence,
                        },
                    },
                )
            };
            if !changed {
                planner.set_event_error("Event could not be saved.".into());
                return;
            }
            if let Err(error) = oziclock_storage::save(&updated) {
                planner.set_event_error(format!("Save failed: {error}").into());
                return;
            }
            *settings = updated;
            if let Some(id) = edited_id {
                queue_for_save_event
                    .borrow_mut()
                    .retain(|item| item.event_id != id);
                display_event_attention(
                    &attention_for_save_event,
                    &owner_for_save_event,
                    &queue_for_save_event,
                );
            }
            let (events, all_day_events) =
                plan_event_rows(&settings, *start_for_save_event.borrow());
            event_model_for_save_event.set_vec(events);
            all_day_model_for_save_event.set_vec(all_day_events);
            planner.set_selected_plan_event("".into());
            planner.set_editing_event("".into());
            planner.set_editor_modal(-1);
        },
    );
    let planner_for_delete_event = planner_window.as_weak();
    let settings_for_delete_event = shared_settings.clone();
    let event_model_for_delete_event = plan_event_model.clone();
    let all_day_model_for_delete_event = plan_all_day_event_model.clone();
    let start_for_delete_event = plan_week_start.clone();
    let queue_for_delete_event = event_attention_queue.clone();
    let attention_for_delete_event = event_attention_window.as_weak();
    let owner_for_delete_event = window.as_weak();
    planner_window.on_request_delete_event(move |event_id| {
        let Some(id) = PlannerId::new(event_id.to_string()) else {
            return;
        };
        let mut settings = settings_for_delete_event.borrow_mut();
        let mut updated = settings.clone();
        if !execute_planner_command(
            &mut updated.planner,
            PlannerCommand::DeleteEvent { id: id.clone() },
        ) {
            return;
        }
        if let Err(error) = oziclock_storage::save(&updated) {
            if let Some(planner) = planner_for_delete_event.upgrade() {
                planner.set_event_error(format!("Delete failed: {error}").into());
            }
            return;
        }
        *settings = updated;
        queue_for_delete_event
            .borrow_mut()
            .retain(|item| item.event_id != id);
        display_event_attention(
            &attention_for_delete_event,
            &owner_for_delete_event,
            &queue_for_delete_event,
        );
        let (events, all_day_events) = plan_event_rows(&settings, *start_for_delete_event.borrow());
        event_model_for_delete_event.set_vec(events);
        all_day_model_for_delete_event.set_vec(all_day_events);
        if let Some(planner) = planner_for_delete_event.upgrade() {
            planner.set_selected_plan_event("".into());
            planner.set_selected_plan_date("".into());
            planner.set_selected_plan_title("".into());
            planner.set_selected_plan_time("".into());
            planner.set_editing_event("".into());
            planner.set_editor_modal(-1);
        }
    });
    let queue_for_event_dismiss = event_attention_queue.clone();
    let sound_for_event_dismiss = alert_sound.clone();
    let settings_for_event_dismiss = shared_settings.clone();
    let attention_for_event_dismiss = event_attention_window.as_weak();
    let owner_for_event_dismiss = window.as_weak();
    event_attention_window.on_request_dismiss(move || {
        let Some(item) = queue_for_event_dismiss.borrow().front().cloned() else {
            return;
        };
        let mut settings = settings_for_event_dismiss.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::AcknowledgeEvent {
                id: item.event_id,
                occurrence_utc: item.occurrence_utc,
                acknowledged_at_utc: Utc::now().to_rfc3339(),
            },
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            sound_for_event_dismiss.stop();
            queue_for_event_dismiss.borrow_mut().pop_front();
            display_event_attention(
                &attention_for_event_dismiss,
                &owner_for_event_dismiss,
                &queue_for_event_dismiss,
            );
        }
    });
    display_event_attention(
        &event_attention_window.as_weak(),
        &window.as_weak(),
        &event_attention_queue,
    );
    schedule_event_attention_pulse(Rc::new(Timer::default()), event_attention_window.as_weak());
    schedule_event_refresh(
        alert_sound.clone(),
        Rc::new(Timer::default()),
        shared_settings.clone(),
        event_attention_window.as_weak(),
        window.as_weak(),
        event_attention_queue,
    );
    let task_attention_queue = Rc::new(RefCell::new(pending_task_attention_items(
        &shared_settings.borrow(),
    )));
    let queue_for_task_dismiss = task_attention_queue.clone();
    let sound_for_task_dismiss = alert_sound.clone();
    let settings_for_task_dismiss = shared_settings.clone();
    let attention_for_task_dismiss = task_attention_window.as_weak();
    let owner_for_task_dismiss = window.as_weak();
    task_attention_window.on_request_dismiss(move || {
        let Some(item) = queue_for_task_dismiss.borrow().front().cloned() else {
            return;
        };
        let mut settings = settings_for_task_dismiss.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::DismissTaskAlert { id: item.task_id },
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            sound_for_task_dismiss.stop();
            queue_for_task_dismiss.borrow_mut().pop_front();
            display_task_attention(
                &attention_for_task_dismiss,
                &owner_for_task_dismiss,
                &queue_for_task_dismiss,
            );
        }
    });
    display_task_attention(
        &task_attention_window.as_weak(),
        &window.as_weak(),
        &task_attention_queue,
    );
    schedule_task_attention_pulse(Rc::new(Timer::default()), task_attention_window.as_weak());
    schedule_task_refresh(
        alert_sound.clone(),
        Rc::new(Timer::default()),
        shared_settings.clone(),
        task_attention_window.as_weak(),
        window.as_weak(),
        task_attention_queue.clone(),
    );
    let planner_task_model = Rc::new(VecModel::from(planner_task_rows(&shared_settings.borrow())));
    let planner_completed_task_model = Rc::new(VecModel::from(planner_completed_task_rows(
        &shared_settings.borrow(),
    )));
    planner_window.set_tasks(ModelRc::from(planner_task_model.clone()));
    planner_window.set_completed_tasks(ModelRc::from(planner_completed_task_model.clone()));
    let planner_for_drag = planner_window.as_weak();
    planner_window.on_request_window_drag(move || {
        if let Some(planner) = planner_for_drag.upgrade() {
            let _ = planner
                .window()
                .with_winit_window(|window| window.drag_window());
        }
    });
    let planner_for_close = planner_window.as_weak();
    planner_window.on_request_close(move || {
        if let Some(planner) = planner_for_close.upgrade() {
            let _ = planner.hide();
        }
    });
    let planner_for_add_task = planner_window.as_weak();
    let settings_for_add_task = shared_settings.clone();
    let task_model_for_add_task = planner_task_model.clone();
    let completed_model_for_add_task = planner_completed_task_model.clone();
    let queue_for_add_task = task_attention_queue.clone();
    let attention_for_add_task = task_attention_window.as_weak();
    let owner_for_add_task = window.as_weak();
    planner_window.on_request_add_task(move |title, scheduled, date, time, alert| {
        let Some(planner) = planner_for_add_task.upgrade() else {
            return;
        };
        planner.set_task_error("".into());
        let title = title.trim();
        if title.is_empty() {
            planner.set_task_error("Enter a task name.".into());
            return;
        }
        let mut settings = settings_for_add_task.borrow_mut();
        let due_utc = if scheduled {
            let zone = main_time_zone(&settings);
            let Some(due) = oziclock_app::reminder_time::resolve_local(&date, &time, &zone) else {
                planner.set_task_error("Enter a valid date and time.".into());
                return;
            };
            Some(due.to_rfc3339())
        } else {
            None
        };
        let alert_offset = parse_alert_with_custom(
            &alert,
            &planner.get_task_custom_alert_minutes(),
            "At due time",
        );
        let Some(alert_offset) = alert_offset else {
            planner.set_task_error("Select a valid alert.".into());
            return;
        };
        let alerts = if scheduled {
            alert_offset
                .map(|offset_minutes| {
                    vec![AlertRule {
                        id: PlannerId::new(format!(
                            "task-alert-{}",
                            Utc::now().timestamp_nanos_opt().unwrap_or_default()
                        ))
                        .expect("generated alert id is valid"),
                        offset_minutes,
                        play_sound: true,
                    }]
                })
                .unwrap_or_default()
        } else {
            vec![]
        };
        let mut updated = settings.clone();
        let editing = if planner.get_selected_section() == 5 {
            planner.get_editing_task()
        } else {
            "".into()
        };
        let changed = if editing.is_empty() {
            let id = PlannerId::new(format!(
                "task-{}",
                Utc::now().timestamp_nanos_opt().unwrap_or_default()
            ))
            .expect("generated task id is valid");
            updated.planner.tasks.push(Task {
                id,
                title: title.into(),
                status: TaskStatus::Open,
                due_utc,
                scheduled_start_utc: None,
                scheduled_end_utc: None,
                color: None,
                tags: vec![],
                notes: None,
                alerts,
                attention_pending: false,
                attention_due_utc: None,
                delivered_for_due_utc: None,
            });
            true
        } else {
            let Some(id) = PlannerId::new(editing.to_string()) else {
                return;
            };
            execute_planner_command(
                &mut updated.planner,
                PlannerCommand::UpdateTask {
                    id,
                    title: title.into(),
                    due_utc,
                    alerts,
                },
            )
        };
        if !changed {
            return;
        }
        if let Err(error) = oziclock_storage::save(&updated) {
            planner.set_task_error(format!("Save failed: {error}").into());
            return;
        }
        *settings = updated;
        if !editing.is_empty() {
            queue_for_add_task
                .borrow_mut()
                .retain(|item| item.task_id.to_string() != editing.as_str());
            display_task_attention(
                &attention_for_add_task,
                &owner_for_add_task,
                &queue_for_add_task,
            );
        }
        refresh_task_models(
            &planner,
            &settings,
            &task_model_for_add_task,
            &completed_model_for_add_task,
        );
        planner.set_task_draft("".into());
        planner.set_task_scheduled(false);
        planner.set_task_alert("No alert".into());
        planner.set_editing_task("".into());
        planner.set_selected_task("".into());
        planner.set_editor_modal(-1);
    });
    let planner_for_complete_task = planner_window.as_weak();
    let settings_for_complete_task = shared_settings.clone();
    let task_model_for_complete_task = planner_task_model.clone();
    let completed_model_for_complete_task = planner_completed_task_model.clone();
    let queue_for_complete_task = task_attention_queue.clone();
    let attention_for_complete_task = task_attention_window.as_weak();
    let owner_for_complete_task = window.as_weak();
    planner_window.on_request_complete_task(move |task_id| {
        let Some(id) = PlannerId::new(task_id.to_string()) else {
            return;
        };
        let mut settings = settings_for_complete_task.borrow_mut();
        let mut updated = settings.clone();
        if !execute_planner_command(&mut updated.planner, PlannerCommand::CompleteTask { id })
            || oziclock_storage::save(&updated).is_err()
        {
            return;
        }
        *settings = updated;
        queue_for_complete_task
            .borrow_mut()
            .retain(|item| item.task_id.to_string() != task_id.as_str());
        display_task_attention(
            &attention_for_complete_task,
            &owner_for_complete_task,
            &queue_for_complete_task,
        );
        if let Some(planner) = planner_for_complete_task.upgrade() {
            refresh_task_models(
                &planner,
                &settings,
                &task_model_for_complete_task,
                &completed_model_for_complete_task,
            );
            planner.set_selected_task("".into());
            planner.set_editing_task("".into());
        }
    });
    let settings_for_edit_task = shared_settings.clone();
    let planner_for_edit_task = planner_window.as_weak();
    planner_window.on_request_edit_task(move |task_id| {
        let settings = settings_for_edit_task.borrow();
        let Some(task) = settings.planner.tasks.iter().find(|task| {
            task.id.to_string() == task_id.as_str() && task.status == TaskStatus::Open
        }) else {
            return;
        };
        let Some(planner) = planner_for_edit_task.upgrade() else {
            return;
        };
        planner.set_editing_task(task_id);
        planner.set_task_draft(task.title.clone().into());
        planner.set_task_scheduled(task.due_utc.is_some());
        if let Some(due) = task
            .due_utc
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        {
            let zone = main_time_zone(&settings)
                .parse::<Tz>()
                .unwrap_or(chrono_tz::UTC);
            let local = due.with_timezone(&zone);
            planner.set_task_date_draft(local.format("%Y-%m-%d").to_string().into());
            planner.set_task_time_draft(local.format("%H:%M").to_string().into());
        }
        planner.set_task_alert(
            task.alerts
                .first()
                .map(|rule| task_alert_label(rule.offset_minutes))
                .unwrap_or("No alert")
                .into(),
        );
        if let Some(alert) = task
            .alerts
            .first()
            .filter(|alert| !matches!(alert.offset_minutes, 0 | 5 | 15 | 30 | 60))
        {
            planner.set_task_custom_alert_minutes(alert.offset_minutes.to_string().into());
        }
        planner.set_task_error("".into());
    });
    let settings_for_reopen_task = shared_settings.clone();
    let planner_for_reopen_task = planner_window.as_weak();
    let open_model_for_reopen_task = planner_task_model.clone();
    let completed_model_for_reopen_task = planner_completed_task_model.clone();
    planner_window.on_request_reopen_task(move |task_id| {
        apply_task_command(
            &settings_for_reopen_task,
            &planner_for_reopen_task,
            &open_model_for_reopen_task,
            &completed_model_for_reopen_task,
            PlannerCommand::ReopenTask {
                id: match PlannerId::new(task_id.to_string()) {
                    Some(id) => id,
                    None => return,
                },
            },
        );
    });
    let settings_for_archive_task = shared_settings.clone();
    let planner_for_archive_task = planner_window.as_weak();
    let open_model_for_archive_task = planner_task_model.clone();
    let completed_model_for_archive_task = planner_completed_task_model.clone();
    planner_window.on_request_archive_task(move |task_id| {
        apply_task_command(
            &settings_for_archive_task,
            &planner_for_archive_task,
            &open_model_for_archive_task,
            &completed_model_for_archive_task,
            PlannerCommand::ArchiveTask {
                id: match PlannerId::new(task_id.to_string()) {
                    Some(id) => id,
                    None => return,
                },
            },
        );
    });
    let settings_for_delete_task = shared_settings.clone();
    let planner_for_delete_task = planner_window.as_weak();
    let open_model_for_delete_task = planner_task_model.clone();
    let completed_model_for_delete_task = planner_completed_task_model.clone();
    planner_window.on_request_delete_task(move |task_id| {
        apply_task_command(
            &settings_for_delete_task,
            &planner_for_delete_task,
            &open_model_for_delete_task,
            &completed_model_for_delete_task,
            PlannerCommand::DeleteTask {
                id: match PlannerId::new(task_id.to_string()) {
                    Some(id) => id,
                    None => return,
                },
            },
        );
    });
    planner_alarm_bindings::wire_alarm_bindings(
        alert_sound.clone(),
        &planner_window,
        &alarm_attention_window,
        &window,
        shared_settings.clone(),
    );
    planner_timer_bindings::wire_timer_bindings(
        alert_sound.clone(),
        &planner_window,
        &timer_attention_window,
        &window,
        shared_settings.clone(),
    );
    let planner_reminder_model = Rc::new(VecModel::from(planner_reminder_rows(
        &shared_settings.borrow(),
    )));
    planner_window.set_reminders(ModelRc::from(planner_reminder_model.clone()));
    let reminder_attention_queue = Rc::new(RefCell::new(pending_reminder_attention_items(
        &shared_settings.borrow(),
    )));
    let queue_for_reminder_dismiss = reminder_attention_queue.clone();
    let sound_for_reminder_dismiss = alert_sound.clone();
    let settings_for_reminder_dismiss = shared_settings.clone();
    let model_for_reminder_dismiss = planner_reminder_model.clone();
    let attention_for_reminder_dismiss = reminder_attention_window.as_weak();
    let owner_for_reminder_dismiss = window.as_weak();
    reminder_attention_window.on_request_dismiss(move || {
        let Some(item) = queue_for_reminder_dismiss.borrow().front().cloned() else {
            return;
        };
        let mut settings = settings_for_reminder_dismiss.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::DismissReminder {
                id: item.reminder_id,
            },
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            sound_for_reminder_dismiss.stop();
            model_for_reminder_dismiss.set_vec(planner_reminder_rows(&settings));
            queue_for_reminder_dismiss.borrow_mut().pop_front();
            display_reminder_attention(
                &attention_for_reminder_dismiss,
                &owner_for_reminder_dismiss,
                &queue_for_reminder_dismiss,
            );
        }
    });
    display_reminder_attention(
        &reminder_attention_window.as_weak(),
        &window.as_weak(),
        &reminder_attention_queue,
    );
    schedule_reminder_attention_pulse(
        Rc::new(Timer::default()),
        reminder_attention_window.as_weak(),
    );
    let planner_for_adjust_reminder = planner_window.as_weak();
    planner_window.on_request_adjust_reminder_part(move |part, direction| {
        let Some(planner) = planner_for_adjust_reminder.upgrade() else {
            return;
        };
        let text = match part {
            1 => planner.get_reminder_hours_draft(),
            2 => planner.get_reminder_minutes_draft(),
            _ => return,
        };
        let Some(value) = adjust_timer_part(&text, part, direction) else {
            return;
        };
        let value = format!("{value:02}").into();
        if part == 1 {
            planner.set_reminder_hours_draft(value);
        } else {
            planner.set_reminder_minutes_draft(value);
        }
    });
    let planner_for_edit_reminder = planner_window.as_weak();
    planner_window.on_request_edit_reminder_part(move |part, text| {
        let Some(planner) = planner_for_edit_reminder.upgrade() else {
            return;
        };
        let Some(value) = mask_timer_part(&text, part) else {
            return;
        };
        if part == 1 {
            planner.set_reminder_hours_draft(value.into());
        } else if part == 2 {
            planner.set_reminder_minutes_draft(value.into());
        }
    });
    let planner_for_add_reminder = planner_window.as_weak();
    let settings_for_add_reminder = shared_settings.clone();
    let model_for_add_reminder = planner_reminder_model.clone();
    let plan_model_for_add_reminder = plan_reminder_model.clone();
    let plan_start_for_add_reminder = plan_week_start.clone();
    let queue_for_add_reminder = reminder_attention_queue.clone();
    let attention_for_add_reminder = reminder_attention_window.as_weak();
    let owner_for_add_reminder = window.as_weak();
    planner_window.on_request_add_reminder(
        move |title, date, hours, minutes, recurrence, mon, tue, wed, thu, fri, sat, sun| {
            let Some(planner) = planner_for_add_reminder.upgrade() else {
                return;
            };
            planner.set_reminder_error("".into());
            let title = title.trim();
            if title.is_empty() {
                planner.set_reminder_error("Enter a reminder name.".into());
                return;
            }
            let Some(time) = normalize_alarm_time(&format!("{hours}:{minutes}")) else {
                planner.set_reminder_error("Enter a valid time.".into());
                return;
            };
            let mut settings = settings_for_add_reminder.borrow_mut();
            let mut updated = settings.clone();
            let zone = main_time_zone(&updated);
            let Some(due) = oziclock_app::reminder_time::resolve_local(&date, &time, &zone) else {
                planner.set_reminder_error("Enter a valid date.".into());
                return;
            };
            let schedule = match recurrence.as_str() {
                "Once" => ReminderSchedule::Absolute {
                    at_utc: due.to_rfc3339(),
                    source_time_zone: zone,
                },
                "Daily" => oziclock_app::reminder_time::recurring_schedule(
                    ReminderRecurrence::Daily,
                    &date,
                    &time,
                    &zone,
                )
                .expect("validated reminder date and time resolve"),
                "Weekly" => {
                    let weekdays = [mon, tue, wed, thu, fri, sat, sun]
                        .into_iter()
                        .enumerate()
                        .filter_map(|(day, selected)| selected.then_some(day as u8))
                        .collect::<Vec<_>>();
                    if weekdays.is_empty() {
                        planner.set_reminder_error("Select at least one weekday.".into());
                        return;
                    }
                    oziclock_app::reminder_time::recurring_schedule(
                        ReminderRecurrence::Weekly { weekdays },
                        &date,
                        &time,
                        &zone,
                    )
                    .expect("validated reminder date and time resolve")
                }
                "Monthly" => oziclock_app::reminder_time::recurring_schedule(
                    ReminderRecurrence::Monthly {
                        day: due
                            .with_timezone(&zone.parse::<Tz>().expect("main zone is valid"))
                            .day() as u8,
                    },
                    &date,
                    &time,
                    &zone,
                )
                .expect("validated reminder date and time resolve"),
                "Yearly" => {
                    let local = due.with_timezone(&zone.parse::<Tz>().expect("main zone is valid"));
                    oziclock_app::reminder_time::recurring_schedule(
                        ReminderRecurrence::Yearly {
                            month: local.month() as u8,
                            day: local.day() as u8,
                        },
                        &date,
                        &time,
                        &zone,
                    )
                    .expect("validated reminder date and time resolve")
                }
                _ => {
                    planner.set_reminder_error("Choose a recurrence.".into());
                    return;
                }
            };
            if oziclock_app::reminder_time::due_utc(&schedule).is_none_or(|due| due <= Utc::now()) {
                planner.set_reminder_error("Choose a future date and time.".into());
                return;
            }
            let editing = planner.get_editing_reminder();
            let editing_id = PlannerId::new(editing.to_string());
            let changed = if editing.is_empty() {
                updated.planner.reminders.push(Reminder {
                    id: PlannerId::new(format!(
                        "reminder-{}",
                        Utc::now().timestamp_nanos_opt().unwrap_or_default()
                    ))
                    .expect("generated reminder id is valid"),
                    title: title.to_owned(),
                    schedule,
                    alerts: vec![],
                    enabled: true,
                    attention_pending: false,
                    attention_due_utc: None,
                });
                true
            } else {
                let Some(id) = editing_id.clone() else {
                    return;
                };
                execute_planner_command(
                    &mut updated.planner,
                    PlannerCommand::UpdateReminder {
                        id,
                        title: title.to_owned(),
                        schedule,
                    },
                )
            };
            if !changed {
                return;
            }
            if let Err(error) = oziclock_storage::save(&updated) {
                planner.set_reminder_error(format!("Save failed: {error}").into());
                return;
            }
            *settings = updated;
            model_for_add_reminder.set_vec(planner_reminder_rows(&settings));
            plan_model_for_add_reminder.set_vec(plan_reminder_rows(
                &settings,
                *plan_start_for_add_reminder.borrow(),
            ));
            if let Some(id) = editing_id {
                queue_for_add_reminder
                    .borrow_mut()
                    .retain(|item| item.reminder_id != id);
                display_reminder_attention(
                    &attention_for_add_reminder,
                    &owner_for_add_reminder,
                    &queue_for_add_reminder,
                );
            }
            planner.set_editing_reminder("".into());
            planner.set_selected_reminder("".into());
            planner.set_reminder_title_draft("Reminder".into());
            planner.set_reminder_recurrence("Once".into());
            let (date, time) = default_reminder_datetime(&settings);
            planner.set_reminder_date_draft(date.into());
            planner.set_reminder_hours_draft(time[..2].into());
            planner.set_reminder_minutes_draft(time[3..].into());
            planner.set_editor_modal(-1);
        },
    );
    let settings_for_edit_reminder = shared_settings.clone();
    let planner_for_edit_reminder = planner_window.as_weak();
    planner_window.on_request_edit_reminder(move |reminder_id| {
        let settings = settings_for_edit_reminder.borrow();
        let Some(reminder) = settings
            .planner
            .reminders
            .iter()
            .find(|reminder| reminder.id.to_string() == reminder_id.as_str())
        else {
            return;
        };
        let (at_utc, source_time_zone) = match &reminder.schedule {
            ReminderSchedule::Absolute {
                at_utc,
                source_time_zone,
            } => (at_utc, source_time_zone),
            ReminderSchedule::Recurring {
                next_at_utc,
                source_time_zone,
                ..
            } => (next_at_utc, source_time_zone),
            ReminderSchedule::ImportantDate { .. } => return,
        };
        let Some(due) = DateTime::parse_from_rfc3339(at_utc).ok() else {
            return;
        };
        let Some(zone) = source_time_zone.parse::<Tz>().ok() else {
            return;
        };
        let local = due.with_timezone(&zone);
        let Some(planner) = planner_for_edit_reminder.upgrade() else {
            return;
        };
        planner.set_editing_reminder(reminder_id);
        planner.set_reminder_title_draft(reminder.title.clone().into());
        planner.set_reminder_date_draft(local.format("%Y-%m-%d").to_string().into());
        planner.set_reminder_hours_draft(local.format("%H").to_string().into());
        planner.set_reminder_minutes_draft(local.format("%M").to_string().into());
        match &reminder.schedule {
            ReminderSchedule::Absolute { .. } => planner.set_reminder_recurrence("Once".into()),
            ReminderSchedule::Recurring { recurrence, .. } => {
                let mode = match recurrence {
                    ReminderRecurrence::Daily => "Daily",
                    ReminderRecurrence::Weekly { .. } => "Weekly",
                    ReminderRecurrence::Monthly { .. } => "Monthly",
                    ReminderRecurrence::Yearly { .. } => "Yearly",
                };
                planner.set_reminder_recurrence(mode.into());
                if let ReminderRecurrence::Weekly { weekdays } = recurrence {
                    planner.set_reminder_mon(weekdays.contains(&0));
                    planner.set_reminder_tue(weekdays.contains(&1));
                    planner.set_reminder_wed(weekdays.contains(&2));
                    planner.set_reminder_thu(weekdays.contains(&3));
                    planner.set_reminder_fri(weekdays.contains(&4));
                    planner.set_reminder_sat(weekdays.contains(&5));
                    planner.set_reminder_sun(weekdays.contains(&6));
                }
            }
            ReminderSchedule::ImportantDate { .. } => {}
        }
        planner.set_reminder_error("".into());
    });
    let settings_for_toggle_reminder = shared_settings.clone();
    let model_for_toggle_reminder = planner_reminder_model.clone();
    let plan_model_for_toggle_reminder = plan_reminder_model.clone();
    let plan_start_for_toggle_reminder = plan_week_start.clone();
    planner_window.on_request_toggle_reminder(move |reminder_id, enabled| {
        let Some(id) = PlannerId::new(reminder_id.to_string()) else {
            return;
        };
        let mut settings = settings_for_toggle_reminder.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::SetReminderEnabled { id, enabled },
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            model_for_toggle_reminder.set_vec(planner_reminder_rows(&settings));
            plan_model_for_toggle_reminder.set_vec(plan_reminder_rows(
                &settings,
                *plan_start_for_toggle_reminder.borrow(),
            ));
        }
    });
    let settings_for_delete_reminder = shared_settings.clone();
    let model_for_delete_reminder = planner_reminder_model.clone();
    let plan_model_for_delete_reminder = plan_reminder_model.clone();
    let plan_start_for_delete_reminder = plan_week_start.clone();
    let planner_for_delete_reminder = planner_window.as_weak();
    let queue_for_delete_reminder = reminder_attention_queue.clone();
    let attention_for_delete_reminder = reminder_attention_window.as_weak();
    let owner_for_delete_reminder = window.as_weak();
    planner_window.on_request_delete_reminder(move |reminder_id| {
        let Some(id) = PlannerId::new(reminder_id.to_string()) else {
            return;
        };
        let mut settings = settings_for_delete_reminder.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::DeleteReminder { id: id.clone() },
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            model_for_delete_reminder.set_vec(planner_reminder_rows(&settings));
            plan_model_for_delete_reminder.set_vec(plan_reminder_rows(
                &settings,
                *plan_start_for_delete_reminder.borrow(),
            ));
            queue_for_delete_reminder
                .borrow_mut()
                .retain(|item| item.reminder_id != id);
            display_reminder_attention(
                &attention_for_delete_reminder,
                &owner_for_delete_reminder,
                &queue_for_delete_reminder,
            );
            if let Some(planner) = planner_for_delete_reminder.upgrade() {
                planner.set_selected_reminder("".into());
                planner.set_editing_reminder("".into());
            }
        }
    });
    schedule_reminder_refresh(
        alert_sound.clone(),
        Rc::new(Timer::default()),
        planner_reminder_model,
        shared_settings.clone(),
        reminder_attention_window.as_weak(),
        window.as_weak(),
        reminder_attention_queue,
    );
    let stopwatch_started_at = Rc::new(Cell::new(None::<Instant>));
    if shared_settings
        .borrow()
        .planner
        .stopwatch
        .as_ref()
        .is_some_and(|stopwatch| stopwatch.state == StopwatchState::Running)
    {
        shared_settings
            .borrow_mut()
            .planner
            .stopwatch
            .as_mut()
            .expect("running stopwatch exists")
            .interrupt();
    }
    let stopwatch_lap_model = Rc::new(VecModel::from(stopwatch_lap_rows(
        &shared_settings.borrow(),
    )));
    planner_window.set_stopwatch_laps(ModelRc::from(stopwatch_lap_model.clone()));
    refresh_stopwatch_ui(
        &planner_window,
        &shared_settings.borrow(),
        None,
        &stopwatch_lap_model,
    );
    let stopwatch_for_toggle = planner_window.as_weak();
    let settings_for_stopwatch_toggle = shared_settings.clone();
    let started_for_stopwatch_toggle = stopwatch_started_at.clone();
    let laps_for_stopwatch_toggle = stopwatch_lap_model.clone();
    planner_window.on_request_toggle_stopwatch(move || {
        let mut settings = settings_for_stopwatch_toggle.borrow_mut();
        if settings.planner.stopwatch.is_none() {
            settings.planner.stopwatch = Some(Stopwatch {
                state: StopwatchState::Idle,
                elapsed_seconds: 0,
                elapsed_milliseconds: 0,
                laps_seconds: vec![],
                laps_milliseconds: vec![],
            });
        }
        let elapsed_milliseconds =
            stopwatch_elapsed_milliseconds(&settings, started_for_stopwatch_toggle.get());
        let command = if settings
            .planner
            .stopwatch
            .as_ref()
            .is_some_and(|stopwatch| stopwatch.state == StopwatchState::Running)
        {
            started_for_stopwatch_toggle.set(None);
            PlannerCommand::PauseStopwatch {
                elapsed_milliseconds: elapsed_milliseconds as u64,
            }
        } else {
            started_for_stopwatch_toggle.set(Some(Instant::now()));
            PlannerCommand::StartStopwatch
        };
        if execute_planner_command(&mut settings.planner, command) {
            let _ = oziclock_storage::save(&settings);
            if let Some(planner) = stopwatch_for_toggle.upgrade() {
                refresh_stopwatch_ui(
                    &planner,
                    &settings,
                    started_for_stopwatch_toggle.get(),
                    &laps_for_stopwatch_toggle,
                );
            }
        }
    });
    let stopwatch_for_reset = planner_window.as_weak();
    let settings_for_stopwatch_reset = shared_settings.clone();
    let started_for_stopwatch_reset = stopwatch_started_at.clone();
    let laps_for_stopwatch_reset = stopwatch_lap_model.clone();
    planner_window.on_request_reset_stopwatch(move || {
        let mut settings = settings_for_stopwatch_reset.borrow_mut();
        started_for_stopwatch_reset.set(None);
        if execute_planner_command(&mut settings.planner, PlannerCommand::ResetStopwatch) {
            let _ = oziclock_storage::save(&settings);
            if let Some(planner) = stopwatch_for_reset.upgrade() {
                refresh_stopwatch_ui(&planner, &settings, None, &laps_for_stopwatch_reset);
            }
        }
    });
    let stopwatch_for_lap = planner_window.as_weak();
    let settings_for_stopwatch_lap = shared_settings.clone();
    let started_for_stopwatch_lap = stopwatch_started_at.clone();
    let laps_for_stopwatch_lap = stopwatch_lap_model.clone();
    planner_window.on_request_stopwatch_lap(move || {
        let mut settings = settings_for_stopwatch_lap.borrow_mut();
        let elapsed_milliseconds =
            stopwatch_elapsed_milliseconds(&settings, started_for_stopwatch_lap.get()) as u64;
        if execute_planner_command(
            &mut settings.planner,
            PlannerCommand::RecordStopwatchLap {
                elapsed_milliseconds,
            },
        ) {
            let _ = oziclock_storage::save(&settings);
            if let Some(planner) = stopwatch_for_lap.upgrade() {
                refresh_stopwatch_ui(
                    &planner,
                    &settings,
                    started_for_stopwatch_lap.get(),
                    &laps_for_stopwatch_lap,
                );
            }
        }
    });
    let stopwatch_for_undo = planner_window.as_weak();
    let settings_for_stopwatch_undo = shared_settings.clone();
    let started_for_stopwatch_undo = stopwatch_started_at.clone();
    let laps_for_stopwatch_undo = stopwatch_lap_model.clone();
    planner_window.on_request_undo_stopwatch_lap(move || {
        let mut settings = settings_for_stopwatch_undo.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(&mut updated.planner, PlannerCommand::UndoStopwatchLap)
            && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            if let Some(planner) = stopwatch_for_undo.upgrade() {
                refresh_stopwatch_ui(
                    &planner,
                    &settings,
                    started_for_stopwatch_undo.get(),
                    &laps_for_stopwatch_undo,
                );
            }
        }
    });
    let stopwatch_for_clear = planner_window.as_weak();
    let settings_for_stopwatch_clear = shared_settings.clone();
    let started_for_stopwatch_clear = stopwatch_started_at.clone();
    let laps_for_stopwatch_clear = stopwatch_lap_model.clone();
    planner_window.on_request_clear_stopwatch_laps(move || {
        let mut settings = settings_for_stopwatch_clear.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(&mut updated.planner, PlannerCommand::ClearStopwatchLaps)
            && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            if let Some(planner) = stopwatch_for_clear.upgrade() {
                refresh_stopwatch_ui(
                    &planner,
                    &settings,
                    started_for_stopwatch_clear.get(),
                    &laps_for_stopwatch_clear,
                );
            }
        }
    });
    schedule_stopwatch_refresh(
        Rc::new(Timer::default()),
        planner_window.as_weak(),
        shared_settings.clone(),
        stopwatch_started_at,
        stopwatch_lap_model,
    );
    let initial_calendar_now = calendar_local_now(&shared_settings.borrow());
    let initial_calendar_date = CalendarDate::new(
        initial_calendar_now.year(),
        initial_calendar_now.month(),
        initial_calendar_now.day(),
    )
    .expect("current local date is valid");
    let mut initial_calendar_state = CalendarState::new(initial_calendar_date);
    initial_calendar_state.light_theme = shared_settings.borrow().calendar_light_theme;
    initial_calendar_state.monday_first = shared_settings.borrow().calendar_monday_first;
    let calendar_state = Rc::new(RefCell::new(initial_calendar_state));
    let accent = calendar_accent(&shared_settings.borrow());
    calendar_window.set_accent(accent);
    calendar_window.set_accent_foreground(accent_foreground(accent));
    calendar_window
        .set_corner_radius(shared_settings.borrow().corner_radius.clamp(0.0, 15.5) as f32);
    refresh_calendar_window(
        &calendar_window,
        &calendar_state.borrow(),
        initial_calendar_date,
        initial_calendar_now,
    );
    calendar_window.set_week_scroll_y(initial_week_scroll_y(initial_calendar_now));
    let explored_time = Rc::new(RefCell::new(None::<DateTime<Utc>>));
    let clock_timer = Rc::new(Timer::default());
    let ruler_label_settings = shared_settings.clone();
    window.on_format_label(move |column_index, hour| {
        format_ruler_label(&ruler_label_settings.borrow(), column_index, hour).into()
    });
    let main_window_for_ruler = window.as_weak();
    let content_for_ruler = window.as_weak();
    let settings_for_ruler = shared_settings.clone();
    let explored_time_for_ruler = explored_time.clone();
    window.on_request_focus_progress(move |progress| {
        let time_step = (progress * 288.0).round();
        if let Some(content) = content_for_ruler.upgrade() {
            content.set_focus_progress(progress);
            content.set_time_step(time_step);
        }
        let selected_time = ruler_time_step_to_utc(&settings_for_ruler.borrow(), time_step);
        *explored_time_for_ruler.borrow_mut() = Some(selected_time);
        if let Some(main_window) = main_window_for_ruler.upgrade() {
            let settings = settings_for_ruler.borrow();
            update_clock_tiles_at(
                &main_window,
                &settings.clocks_settings,
                settings.show_seconds,
                selected_time,
            );
        }
    });
    let content_for_slider = window.as_weak();
    let main_window_for_slider = window.as_weak();
    let settings_for_slider = shared_settings.clone();
    let explored_time_for_slider = explored_time.clone();
    window.on_request_time_step(move |time_step| {
        if let Some(content) = content_for_slider.upgrade() {
            content.set_focus_progress((time_step / 288.0).clamp(0.0, 1.0));
        }
        let selected_time = ruler_time_step_to_utc(&settings_for_slider.borrow(), time_step);
        *explored_time_for_slider.borrow_mut() = Some(selected_time);
        if let Some(main_window) = main_window_for_slider.upgrade() {
            let settings = settings_for_slider.borrow();
            update_clock_tiles_at(
                &main_window,
                &settings.clocks_settings,
                settings.show_seconds,
                selected_time,
            );
        }
    });
    let content_for_focus = window.as_weak();
    window.on_request_focus_column(move |column_index| {
        if let Some(content) = content_for_focus.upgrade() {
            let maximum_index = content.get_rulers().row_count().saturating_sub(1) as i32;
            content.set_focused_column(column_index.clamp(0, maximum_index));
        }
    });
    if shared_settings.borrow().show_rulers {
        window.invoke_request_focus_progress(window.get_focus_progress());
    }
    let saved_settings = Rc::new(RefCell::new(shared_settings.borrow().clone()));
    let weak_settings_window = settings_window.as_weak();
    let settings_for_open = shared_settings.clone();
    let main_window_for_settings = window.as_weak();
    window.on_request_open_settings(move || {
        if let Some(settings_window) = weak_settings_window.upgrade() {
            open_settings_window(
                &settings_window,
                &main_window_for_settings,
                &settings_for_open.borrow(),
            );
        }
    });
    let weak_settings_window = settings_window.as_weak();
    let settings_for_close = shared_settings.clone();
    let main_window_for_settings_close = window.as_weak();
    settings_window.on_request_close(move || {
        if let Some(settings_window) = weak_settings_window.upgrade() {
            let mut settings = settings_for_close.borrow_mut();
            persist_settings_window_size(&settings_window, &mut settings);
            let _ = oziclock_storage::save(&settings);
            let _ = settings_window.hide();
        }
        set_main_window_modal(&main_window_for_settings_close, false);
    });
    let drag_settings_window = settings_window.as_weak();
    settings_window.on_request_window_drag(move || {
        if let Some(settings_window) = drag_settings_window.upgrade() {
            let _ = settings_window
                .window()
                .with_winit_window(|window| window.drag_window());
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    settings_window.on_request_select(move |index| {
        if let Some(editor) = editor.upgrade() {
            select_clock(&editor, &state.borrow().clocks_settings, index);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_select_general(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_selected_section(0);
            editor.set_color_picker_open(false);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_select_calendar(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_selected_section(2);
            editor.set_color_picker_open(false);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_select_planner(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_selected_section(3);
            editor.set_color_picker_open(false);
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
    settings_window.on_request_add(move || {
        let mut state = state.borrow_mut();
        execute_clock_command(
            &mut state.clocks_settings,
            ClockCommand::Add(ClockSettings {
                label: "UTC".into(),
                time_zone: "UTC".into(),
                color: "#FFFFFFFF".into(),
                is_main: false,
            }),
        );
        let index = state.clocks_settings.len() as i32 - 1;
        update_settings_preview(&editor.upgrade().unwrap(), &state.clocks_settings);
        select_clock(&editor.upgrade().unwrap(), &state.clocks_settings, index);
        if let Some(main_window) = main_window.upgrade() {
            update_clock_tiles(&main_window, &state.clocks_settings, state.show_seconds);
            initialize_ruler_content(&main_window, &state);
            sync_main_window_size(&main_window);
        }
    });
    let pending_remove_index = Rc::new(Cell::new(-1));
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let pending_remove_for_request = pending_remove_index.clone();
    settings_window.on_request_remove(move |index| {
        if let Some(editor) = editor.upgrade() {
            let state = state.borrow();
            let Some(clock) = state.clocks_settings.get(index.max(0) as usize) else {
                return;
            };
            if index < 0 || clock.is_main || state.clocks_settings.len() <= 1 {
                editor.set_status_message("The primary or only clock cannot be removed.".into());
                return;
            }
            pending_remove_for_request.set(index);
            editor.set_remove_confirmation_label(clock.label.clone().into());
            editor.set_remove_confirmation_open(true);
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
    let pending_remove_for_confirm = pending_remove_index.clone();
    settings_window.on_request_confirm_remove(move || {
        if let Some(editor) = editor.upgrade() {
            let index = pending_remove_for_confirm.replace(-1);
            let mut state = state.borrow_mut();
            if index >= 0
                && execute_clock_command(
                    &mut state.clocks_settings,
                    ClockCommand::Remove {
                        index: index as usize,
                    },
                )
            {
                update_settings_preview(&editor, &state.clocks_settings);
                select_clock(&editor, &state.clocks_settings, 0);
                if let Some(main_window) = main_window.upgrade() {
                    update_clock_tiles(&main_window, &state.clocks_settings, state.show_seconds);
                    initialize_ruler_content(&main_window, &state);
                    sync_main_window_size(&main_window);
                }
            }
            editor.set_remove_confirmation_open(false);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_cancel_remove(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_remove_confirmation_open(false);
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
    let planner_for_apply = planner_window.as_weak();
    let calendar_for_apply = calendar_window.as_weak();
    settings_window.on_request_apply(move || {
        if let Some(editor) = editor.upgrade() {
            let index = editor.get_selected_index();
            let selected_index = index.max(0) as usize;
            let make_main = editor.get_editor_is_main();
            let (settings, main_clock_changed) = {
                let mut state = state.borrow_mut();
                let previous_main_clock = main_clock_index(&state.clocks_settings);
                if let Some(clock) = state.clocks_settings.get_mut(selected_index) {
                    clock.label = editor.get_editor_label().to_string();
                    clock.time_zone = editor.get_editor_time_zone().to_string();
                    clock.color = editor.get_editor_color().to_string();
                }
                if selected_index < state.clocks_settings.len() {
                    if make_main {
                        execute_clock_command(
                            &mut state.clocks_settings,
                            ClockCommand::SetMain {
                                index: selected_index,
                            },
                        );
                    } else if previous_main_clock == selected_index {
                        editor.set_editor_is_main(true);
                    } else if let Some(clock) = state.clocks_settings.get_mut(selected_index) {
                        clock.is_main = false;
                    }
                }
                let main_clock_changed =
                    previous_main_clock != main_clock_index(&state.clocks_settings);
                (state.clone(), main_clock_changed)
            };
            refresh_auxiliary_accents(&planner_for_apply, &calendar_for_apply, &settings);
            if main_clock_changed {
                update_settings_preview(&editor, &settings.clocks_settings);
                if let Some(main_window) = main_window.upgrade() {
                    initialize_ruler_content(&main_window, &settings);
                    if settings.show_rulers {
                        main_window.invoke_request_focus_progress(main_window.get_focus_progress());
                    }
                }
            }
            if let Some(main_window) = main_window.upgrade() {
                update_clock_tiles(
                    &main_window,
                    &settings.clocks_settings,
                    settings.show_seconds,
                );
            }
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let timer_for_seconds = clock_timer.clone();
    let explored_time_for_seconds = explored_time.clone();
    let calendar_for_seconds = calendar_window.as_weak();
    let calendar_state_for_seconds = calendar_state.clone();
    settings_window.on_request_set_show_seconds(move |show_seconds| {
        {
            let mut state = state.borrow_mut();
            state.show_seconds = show_seconds;
            if let Some(main_window) = main_window.upgrade() {
                main_window.set_show_seconds(show_seconds);
                update_clock_tiles(&main_window, &state.clocks_settings, show_seconds);
            }
        }
        schedule_clock_refresh(
            timer_for_seconds.clone(),
            main_window.clone(),
            state.clone(),
            explored_time_for_seconds.clone(),
            calendar_for_seconds.clone(),
            calendar_state_for_seconds.clone(),
        );
    });
    let compact_animation_generation = Rc::new(Cell::new(0_u64));
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let compact_animation_for_settings = compact_animation_generation.clone();
    settings_window.on_request_set_compact_mode(move |compact_mode| {
        {
            let mut state = state.borrow_mut();
            state.compact_mode = compact_mode;
        }
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_compact_mode(compact_mode);
            animate_compact_mode(
                main_window.as_weak(),
                compact_mode,
                compact_animation_for_settings.clone(),
            );
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let explored_time_for_toggle = explored_time.clone();
    window.on_request_toggle_rulers(move || {
        let settings = {
            let mut state = state.borrow_mut();
            state.show_rulers = !state.show_rulers;
            state.clone()
        };
        let show_rulers = settings.show_rulers;
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_show_rulers(show_rulers);
            if show_rulers {
                initialize_ruler_content(&main_window, &settings);
                main_window.invoke_request_focus_progress(main_window.get_focus_progress());
            }
            sync_main_window_size(&main_window);
        }
        if !show_rulers {
            *explored_time_for_toggle.borrow_mut() = None;
            if let Some(main_window) = main_window.upgrade() {
                update_clock_tiles(
                    &main_window,
                    &settings.clocks_settings,
                    settings.show_seconds,
                );
            }
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let editor = settings_window.as_weak();
    settings_window.on_request_set_clock_scale(move |clock_scale_percent| {
        let clock_scale_percent = normalize_clock_scale_percent(clock_scale_percent);
        let clock_scale = clock_scale_percent / 100.0;
        {
            let mut state = state.borrow_mut();
            state.clock_scale = f64::from(clock_scale);
        }
        if let Some(editor) = editor.upgrade() {
            editor.set_clock_scale_percent(clock_scale_percent);
        }
        if let Some(main_window) = main_window.upgrade() {
            apply_clock_scale(&main_window, clock_scale);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let about_window_for_radius = Rc::new(RefCell::new(None::<slint::Weak<AboutWindow>>));
    let about_for_corner_radius = about_window_for_radius.clone();
    settings_window.on_request_set_corner_radius(move |corner_radius| {
        let corner_radius = corner_radius.clamp(0.0, 15.5);
        state.borrow_mut().corner_radius = f64::from(corner_radius);
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_corner_radius(corner_radius);
        }
        if let Some(about_window) = about_for_corner_radius
            .borrow()
            .as_ref()
            .and_then(slint::Weak::upgrade)
        {
            about_window.set_corner_radius(corner_radius);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    settings_window.on_request_set_soft_clock_style(move |soft_clock_style| {
        state.borrow_mut().soft_clock_style = soft_clock_style;
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_soft_clock_style(soft_clock_style);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let editor = settings_window.as_weak();
    settings_window.on_request_set_border_color(move |border_color| {
        let Some(border_color) = normalize_border_color(border_color.as_str()) else {
            return;
        };
        state.borrow_mut().border_color = border_color.clone();
        let border_color = parse_color(&border_color);
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_border_color(border_color);
        }
        if let Some(editor) = editor.upgrade() {
            editor.set_border_preview_color(border_color);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let editor = settings_window.as_weak();
    settings_window.on_request_set_non_main_dimming(move |non_main_dimming| {
        let non_main_dimming = ((non_main_dimming / 5.0).round() * 5.0).clamp(0.0, 80.0);
        state.borrow_mut().non_main_dimming = f64::from(non_main_dimming);
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_non_main_dimming(non_main_dimming);
        }
        if let Some(editor) = editor.upgrade() {
            editor.set_non_main_dimming(non_main_dimming);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    settings_window.on_request_set_top_most(move |top_most| {
        state.borrow_mut().top_most = top_most;
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_top_most(top_most);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let editor = settings_window.as_weak();
    window.on_request_toggle_top_most(move || {
        let top_most = {
            let mut state = state.borrow_mut();
            state.top_most = !state.top_most;
            state.top_most
        };
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_top_most(top_most);
        }
        if let Some(editor) = editor.upgrade() {
            editor.set_top_most(top_most);
        }
        let _ = oziclock_storage::save(&state.borrow());
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    settings_window.on_request_set_show_in_task_bar(move |show_in_task_bar| {
        state.borrow_mut().show_in_task_bar = show_in_task_bar;
        if let Some(main_window) = main_window.upgrade() {
            set_main_window_taskbar_visibility(&main_window, show_in_task_bar);
        }
    });
    let state = shared_settings.clone();
    settings_window.on_request_set_launch_at_login(move |enabled| {
        if launch_at_login::set_enabled(enabled).is_ok() {
            state.borrow_mut().launch_at_login = enabled;
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let editor = settings_window.as_weak();
    let compact_animation_for_middle_click = compact_animation_generation.clone();
    window.on_request_toggle_compact(move || {
        let compact_mode = {
            let mut state = state.borrow_mut();
            state.compact_mode = !state.compact_mode;
            state.compact_mode
        };
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_compact_mode(compact_mode);
            animate_compact_mode(
                main_window.as_weak(),
                compact_mode,
                compact_animation_for_middle_click.clone(),
            );
        }
        if let Some(editor) = editor.upgrade() {
            editor.set_compact_mode(compact_mode);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    settings_window.on_request_set_opacity(move |opacity_percent| {
        let opacity = (opacity_percent / 100.0).clamp(0.02, 1.0);
        state.borrow_mut().opacity = f64::from(opacity);
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_inactive_opacity(opacity);
        }
    });
    let state = shared_settings.clone();
    let calendar = calendar_window.as_weak();
    let calendar_state_for_theme = calendar_state.clone();
    settings_window.on_request_set_calendar_light_theme(move |light_theme| {
        state.borrow_mut().calendar_light_theme = light_theme;
        calendar_state_for_theme.borrow_mut().light_theme = light_theme;
        if let Some(calendar) = calendar.upgrade() {
            refresh_calendar_from_settings(
                &calendar,
                &calendar_state_for_theme.borrow(),
                &state.borrow(),
            );
        }
    });
    let state = shared_settings.clone();
    let calendar = calendar_window.as_weak();
    let calendar_state_for_week_start = calendar_state.clone();
    settings_window.on_request_set_calendar_monday_first(move |monday_first| {
        state.borrow_mut().calendar_monday_first = monday_first;
        calendar_state_for_week_start.borrow_mut().monday_first = monday_first;
        if let Some(calendar) = calendar.upgrade() {
            refresh_calendar_from_settings(
                &calendar,
                &calendar_state_for_week_start.borrow(),
                &state.borrow(),
            );
        }
    });
    let state = shared_settings.clone();
    settings_window.on_request_set_calendar_hour_range(move |hour_range| {
        state.borrow_mut().calendar_hour_range = hour_range.clamp(0, 2) as u8;
    });
    let state = shared_settings.clone();
    let sound_for_duration_change = alert_sound.clone();
    let editor = settings_window.as_weak();
    settings_window.on_request_set_alert_sound_duration_seconds(move |seconds| {
        let seconds = normalize_alert_sound_duration_seconds(seconds as u8);
        state.borrow_mut().alert_sound_duration_seconds = seconds;
        sound_for_duration_change.stop();
        if let Some(editor) = editor.upgrade() {
            editor.set_alert_sound_duration_seconds(i32::from(seconds));
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_select_time_zone(move |index| {
        if let Some(editor) = editor.upgrade()
            && let Some(time_zone) = selected_time_zone_id(&editor, index)
        {
            editor.set_editor_time_zone(time_zone.clone().into());
            editor.invoke_request_apply();
        }
    });
    let editor = settings_window.as_weak();
    let time_zones_for_filter = time_zone_options.clone();
    settings_window.on_request_filter_time_zones(move |query| {
        if let Some(editor) = editor.upgrade() {
            apply_time_zone_filter(&editor, &time_zones_for_filter, query.as_str());
        }
    });
    let state = shared_settings.clone();
    let saved = saved_settings.clone();
    let editor = settings_window.as_weak();
    let main_window_for_move_up = window.as_weak();
    settings_window.on_request_move_up(move || {
        if let Some(editor) = editor.upgrade() {
            let mut state = state.borrow_mut();
            move_selected_clock(&editor, &mut state.clocks_settings, -1);
            refresh_clock_order(&main_window_for_move_up, &state);
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window_for_move_down = window.as_weak();
    settings_window.on_request_move_down(move || {
        if let Some(editor) = editor.upgrade() {
            let mut state = state.borrow_mut();
            move_selected_clock(&editor, &mut state.clocks_settings, 1);
            refresh_clock_order(&main_window_for_move_down, &state);
        }
    });
    let drag_index = Rc::new(Cell::new(-1));
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let drag_start = drag_index.clone();
    settings_window.on_request_list_press(move |x, y| {
        let index = (y / 43.0).floor() as i32;
        let clock_count = state.borrow().clocks_settings.len() as i32;
        if index < 0 || index >= clock_count {
            return;
        }
        if let Some(editor) = editor.upgrade() {
            if (14.0..40.0).contains(&x) {
                drag_start.set(index);
                editor.set_dragging_index(index);
                editor.set_dragging_offset(0.0);
            } else if x >= 202.0 {
                editor.invoke_request_remove(index);
            } else {
                select_clock(&editor, &state.borrow().clocks_settings, index);
            }
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let drag_move = drag_index.clone();
    let main_window_for_drag = window.as_weak();
    settings_window.on_request_drag_clock(move |_index, y| {
        if let Some(editor) = editor.upgrade() {
            let current = drag_move.get();
            let target = (y / 43.0).floor() as i32;
            if current >= 0 && current != target && target >= 0 {
                let mut state = state.borrow_mut();
                move_clock_to(&editor, &mut state.clocks_settings, current, target);
                refresh_clock_order(&main_window_for_drag, &state);
                drag_move.set(target);
                editor.set_dragging_index(target);
                editor.set_dragging_offset(y - target as f32 * 43.0 - 21.5);
            } else if current >= 0 {
                editor.set_dragging_offset(y - current as f32 * 43.0 - 21.5);
            }
        }
    });
    let editor = settings_window.as_weak();
    let drag_end = drag_index.clone();
    settings_window.on_request_drag_end(move || {
        drag_end.set(-1);
        if let Some(editor) = editor.upgrade() {
            editor.set_dragging_index(-1);
            editor.set_dragging_offset(0.0);
        }
    });
    let hue = Rc::new(Cell::new(220.0_f32));
    let saturation = Rc::new(Cell::new(70.0_f32));
    let value = Rc::new(Cell::new(90.0_f32));
    let pending_color = Rc::new(RefCell::new(String::new()));
    let editor = settings_window.as_weak();
    let pending_color_for_open = pending_color.clone();
    let hue_for_open = hue.clone();
    let saturation_for_open = saturation.clone();
    let value_for_open = value.clone();
    settings_window.on_request_open_color_picker(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_picking_border_color(false);
            let color = editor.get_editor_color().to_string();
            let (selected_hue, selected_saturation, selected_value) = color_to_hsv(&color);
            *pending_color_for_open.borrow_mut() = color;
            hue_for_open.set(selected_hue);
            saturation_for_open.set(selected_saturation);
            value_for_open.set(selected_value);
            editor.set_picker_hue(selected_hue);
            editor.set_picker_saturation(selected_saturation);
            editor.set_picker_value(selected_value);
            editor.set_picker_hue_color(hsv_color(selected_hue, 100.0, 100.0));
            editor.set_color_picker_open(true);
        }
    });
    let editor = settings_window.as_weak();
    let pending_color_for_border_open = pending_color.clone();
    let hue_for_border_open = hue.clone();
    let saturation_for_border_open = saturation.clone();
    let value_for_border_open = value.clone();
    settings_window.on_request_open_border_color_picker(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_picking_border_color(true);
            let color = editor.get_border_color_value().to_string();
            let (selected_hue, selected_saturation, selected_value) = color_to_hsv(&color);
            *pending_color_for_border_open.borrow_mut() = color;
            hue_for_border_open.set(selected_hue);
            saturation_for_border_open.set(selected_saturation);
            value_for_border_open.set(selected_value);
            editor.set_picker_hue(selected_hue);
            editor.set_picker_saturation(selected_saturation);
            editor.set_picker_value(selected_value);
            editor.set_picker_hue_color(hsv_color(selected_hue, 100.0, 100.0));
            editor.set_color_picker_open(true);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_color_confirm(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_color_picker_open(false);
        }
    });
    let editor = settings_window.as_weak();
    let pending_color_for_cancel = pending_color.clone();
    settings_window.on_request_color_cancel(move || {
        if let Some(editor) = editor.upgrade() {
            let color = pending_color_for_cancel.borrow().clone();
            if editor.get_picking_border_color() {
                editor.set_border_preview_color(parse_color(&color));
                editor.set_border_color_value(color.clone().into());
                editor.invoke_request_set_border_color(color.into());
            } else {
                editor.set_editor_preview_color(parse_color(&color));
                editor.set_editor_color(color.into());
                editor.invoke_request_apply();
            }
            editor.set_color_picker_open(false);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_pick_color(move |color| {
        if let Some(editor) = editor.upgrade() {
            if editor.get_picking_border_color() {
                editor.set_border_color_value(color.clone());
                editor.set_border_preview_color(parse_color(&color));
                editor.invoke_request_set_border_color(color);
            } else {
                editor.set_editor_color(color);
                editor.set_editor_preview_color(parse_color(&editor.get_editor_color()));
                editor.invoke_request_apply();
            }
            editor.set_color_picker_open(false);
        }
    });
    let editor = settings_window.as_weak();
    let hue_for_color = hue.clone();
    let saturation_for_color = saturation.clone();
    let value_for_color = value.clone();
    settings_window.on_request_picker_color(move |x, y| {
        if let Some(editor) = editor.upgrade() {
            let s = (x / 378.0 * 100.0).clamp(0.0, 100.0);
            let v = (100.0 - y / 180.0 * 100.0).clamp(0.0, 100.0);
            saturation_for_color.set(s);
            value_for_color.set(v);
            editor.set_picker_saturation(s);
            editor.set_picker_value(v);
            let color = hsv_hex(hue_for_color.get(), s, v);
            let preview = hsv_color(hue_for_color.get(), s, v);
            if editor.get_picking_border_color() {
                editor.set_border_color_value(color.clone().into());
                editor.set_border_preview_color(preview);
                editor.invoke_request_set_border_color(color.into());
            } else {
                editor.set_editor_color(color.into());
                editor.set_editor_preview_color(preview);
                editor.invoke_request_apply();
            }
        }
    });
    let editor = settings_window.as_weak();
    let hue_for_hue = hue.clone();
    let saturation_for_hue = saturation.clone();
    let value_for_hue = value.clone();
    settings_window.on_request_picker_hue(move |x| {
        if let Some(editor) = editor.upgrade() {
            let h = (x / 378.0 * 360.0).clamp(0.0, 360.0);
            hue_for_hue.set(h);
            editor.set_picker_hue(h);
            editor.set_picker_hue_color(hsv_color(h, 100.0, 100.0));
            let color = hsv_hex(h, saturation_for_hue.get(), value_for_hue.get());
            let preview = hsv_color(h, saturation_for_hue.get(), value_for_hue.get());
            if editor.get_picking_border_color() {
                editor.set_border_color_value(color.clone().into());
                editor.set_border_preview_color(preview);
                editor.invoke_request_set_border_color(color.into());
            } else {
                editor.set_editor_color(color.into());
                editor.set_editor_preview_color(preview);
                editor.invoke_request_apply();
            }
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
    let main_window_for_save_modal = window.as_weak();
    let planner_for_save = planner_window.as_weak();
    let calendar_for_save = calendar_window.as_weak();
    settings_window.on_request_save(move || {
        if let Some(editor) = editor.upgrade() {
            let mut state = state.borrow_mut();
            if editor.get_selected_section() != 1 {
                if let Some(main_window) = main_window.upgrade() {
                    persist_main_window_position(&main_window, &mut state);
                }
                persist_settings_window_size(&editor, &mut state);
                match oziclock_storage::save(&state) {
                    Ok(()) => {
                        *saved.borrow_mut() = state.clone();
                        let _ = editor.hide();
                        set_main_window_modal(&main_window_for_save_modal, false);
                    }
                    Err(error) => editor.set_status_message(format!("Save failed: {error}").into()),
                }
                return;
            }
            let index = editor.get_selected_index();
            if index < 0 || editor.get_editor_time_zone().parse::<Tz>().is_err() {
                editor.set_status_message(
                    "Enter a valid IANA time zone, for example Europe/Kyiv.".into(),
                );
                return;
            }
            let item = &mut state.clocks_settings[index as usize];
            item.label = editor.get_editor_label().to_string();
            item.time_zone = editor.get_editor_time_zone().to_string();
            item.color = editor.get_editor_color().to_string();
            let set_as_main = editor.get_editor_is_main();
            item.is_main = set_as_main;
            if set_as_main {
                execute_clock_command(
                    &mut state.clocks_settings,
                    ClockCommand::SetMain {
                        index: index as usize,
                    },
                );
            }
            if let Some(main_window) = main_window.upgrade() {
                persist_main_window_position(&main_window, &mut state);
            }
            persist_settings_window_size(&editor, &mut state);
            refresh_auxiliary_accents(&planner_for_save, &calendar_for_save, &state);
            match oziclock_storage::save(&state) {
                Ok(()) => {
                    *saved.borrow_mut() = state.clone();
                    editor.set_status_message("Saved beside the executable.".into())
                }
                Err(error) => editor.set_status_message(format!("Save failed: {error}").into()),
            };
            update_settings_preview(&editor, &state.clocks_settings);
            if let Some(main_window) = main_window.upgrade() {
                update_clock_tiles(&main_window, &state.clocks_settings, state.show_seconds);
            }
            if editor.get_status_message() == "Saved beside the executable." {
                let _ = editor.hide();
                set_main_window_modal(&main_window_for_save_modal, false);
            }
        }
    });

    let context_menu = ContextMenuWindow::new()?;
    let planner_for_menu = planner_window.as_weak();
    let menu_for_planner = context_menu.as_weak();
    let owner_for_planner = window.as_weak();
    let settings_for_planner_scroll = shared_settings.clone();
    let planner_scroll_initialized = Rc::new(Cell::new(false));
    let initialized_for_planner = planner_scroll_initialized.clone();
    context_menu.on_request_open_planner(move || {
        if let Some(menu) = menu_for_planner.upgrade() {
            let _ = menu.hide();
        }
        if let Some(planner) = planner_for_menu.upgrade()
            && planner.show().is_ok()
        {
            hide_auxiliary_window_from_taskbar(planner.window());
            let _ = planner
                .window()
                .with_winit_window(|native| native.set_minimized(false));
            position_calendar_window(planner.window(), &owner_for_planner);
            focus_auxiliary_window(planner.window());
            if !initialized_for_planner.replace(true) {
                let planner = planner.as_weak();
                let initial_plan_minute = calendar_local_now(&settings_for_planner_scroll.borrow())
                    .time()
                    .num_seconds_from_midnight()
                    / 60;
                let target = -(initial_plan_minute.saturating_sub(300) as f32);
                if let Some(planner) = planner.upgrade() {
                    planner.set_plan_scroll_y(target);
                }
                Timer::single_shot(Duration::from_millis(16), move || {
                    if let Some(planner) = planner.upgrade() {
                        planner.set_plan_scroll_y(target);
                        let _ = planner
                            .window()
                            .with_winit_window(|native| native.request_redraw());
                    }
                });
            }
        }
    });
    let menu_for_dismiss = context_menu.as_weak();
    context_menu
        .window()
        .on_winit_window_event(move |_, event| {
            if matches!(event, WindowEvent::Focused(false))
                && let Some(context_menu) = menu_for_dismiss.upgrade()
            {
                let _ = context_menu.hide();
            }
            if is_escape_key(event) {
                if let Some(context_menu) = menu_for_dismiss.upgrade() {
                    let _ = context_menu.hide();
                }
                return EventResult::PreventDefault;
            }
            EventResult::Propagate
        });
    let weak_context_menu = context_menu.as_weak();
    let weak_window_for_menu = window.as_weak();
    window.on_request_open_menu(move || {
        if let (Some(context_menu), Some(window)) =
            (weak_context_menu.upgrade(), weak_window_for_menu.upgrade())
        {
            show_context_menu(&context_menu, &window);
        }
    });

    let calendar_refresh_generation = Rc::new(Cell::new(0_u64));
    let calendar_recently_lost_focus = Rc::new(Cell::new(false));
    let calendar_for_keyboard = calendar_window.as_weak();
    let calendar_generation_for_keyboard = calendar_refresh_generation.clone();
    let calendar_focus_guard = calendar_recently_lost_focus.clone();
    calendar_window
        .window()
        .on_winit_window_event(move |_, event| {
            let lost_focus = matches!(event, WindowEvent::Focused(false));
            if is_escape_key(event) || matches!(event, WindowEvent::CloseRequested) || lost_focus {
                if let Some(calendar) = calendar_for_keyboard.upgrade() {
                    let _ = calendar.hide();
                }
                if lost_focus {
                    calendar_focus_guard.set(true);
                    let guard = calendar_focus_guard.clone();
                    slint::Timer::single_shot(Duration::from_millis(500), move || {
                        guard.set(false);
                    });
                }
                calendar_generation_for_keyboard
                    .set(calendar_generation_for_keyboard.get().wrapping_add(1));
                return EventResult::PreventDefault;
            }
            EventResult::Propagate
        });
    let calendar_for_close = calendar_window.as_weak();
    let calendar_generation_for_close = calendar_refresh_generation.clone();
    calendar_window.on_request_close(move || {
        if let Some(calendar) = calendar_for_close.upgrade() {
            let _ = calendar.hide();
        }
        calendar_generation_for_close.set(calendar_generation_for_close.get().wrapping_add(1));
    });
    let calendar_for_drag = calendar_window.as_weak();
    calendar_window.on_request_window_drag(move || {
        if let Some(calendar) = calendar_for_drag.upgrade() {
            let _ = calendar
                .window()
                .with_winit_window(|window| window.drag_window());
        }
    });
    let calendar_for_view = calendar_window.as_weak();
    let main_for_calendar_view = window.as_weak();
    let calendar_state_for_view = calendar_state.clone();
    let settings_for_calendar_view = shared_settings.clone();
    calendar_window.on_request_view(move |view| {
        if let Some(calendar) = calendar_for_view.upgrade() {
            let now = calendar_local_now(&settings_for_calendar_view.borrow());
            let next_view = CalendarView::from_index(view);
            {
                let mut state = calendar_state_for_view.borrow_mut();
                if next_view == CalendarView::Week {
                    state.show_week(
                        CalendarDate::new(now.year(), now.month(), now.day())
                            .expect("current date is valid"),
                    );
                } else {
                    state.view = next_view;
                }
            }
            refresh_calendar_from_settings(
                &calendar,
                &calendar_state_for_view.borrow(),
                &settings_for_calendar_view.borrow(),
            );
            if next_view == CalendarView::Week {
                calendar.set_week_scroll_y(initial_week_scroll_y(now));
            }
            let calendar_for_layout = calendar.as_weak();
            let owner_for_layout = main_for_calendar_view.clone();
            Timer::single_shot(Duration::from_millis(16), move || {
                if let Some(calendar) = calendar_for_layout.upgrade() {
                    position_calendar_window(calendar.window(), &owner_for_layout);
                }
            });
        }
    });
    let calendar_for_theme_toggle = calendar_window.as_weak();
    let calendar_state_for_theme_toggle = calendar_state.clone();
    let settings_for_theme_toggle = shared_settings.clone();
    calendar_window.on_request_toggle_theme(move || {
        let mut settings = settings_for_theme_toggle.borrow_mut();
        settings.calendar_light_theme = !settings.calendar_light_theme;
        let mut state = calendar_state_for_theme_toggle.borrow_mut();
        state.light_theme = settings.calendar_light_theme;
        if let Some(calendar) = calendar_for_theme_toggle.upgrade() {
            calendar.set_light_theme(state.light_theme);
            refresh_calendar_window(
                &calendar,
                &state,
                state.selected,
                calendar_local_now(&settings),
            );
        }
    });
    let calendar_for_previous = calendar_window.as_weak();
    let calendar_state_for_previous = calendar_state.clone();
    let settings_for_calendar_previous = shared_settings.clone();
    calendar_window.on_request_previous(move || {
        if let Some(calendar) = calendar_for_previous.upgrade() {
            calendar_state_for_previous.borrow_mut().navigate(-1);
            refresh_calendar_from_settings(
                &calendar,
                &calendar_state_for_previous.borrow(),
                &settings_for_calendar_previous.borrow(),
            );
        }
    });
    let calendar_for_next = calendar_window.as_weak();
    let calendar_state_for_next = calendar_state.clone();
    let settings_for_calendar_next = shared_settings.clone();
    calendar_window.on_request_next(move || {
        if let Some(calendar) = calendar_for_next.upgrade() {
            calendar_state_for_next.borrow_mut().navigate(1);
            refresh_calendar_from_settings(
                &calendar,
                &calendar_state_for_next.borrow(),
                &settings_for_calendar_next.borrow(),
            );
        }
    });
    let calendar_for_today = calendar_window.as_weak();
    let calendar_state_for_today = calendar_state.clone();
    let settings_for_calendar_today = shared_settings.clone();
    calendar_window.on_request_today(move || {
        if let Some(calendar) = calendar_for_today.upgrade() {
            let now = calendar_local_now(&settings_for_calendar_today.borrow());
            let today = CalendarDate::new(now.year(), now.month(), now.day())
                .expect("current date is valid");
            let mut state = calendar_state_for_today.borrow_mut();
            state.selected = today;
            if state.view == CalendarView::Week {
                state.show_week(today);
            } else {
                state.cursor = today;
            }
            refresh_calendar_window(&calendar, &state, today, now);
            if state.view == CalendarView::Week {
                calendar.set_week_scroll_y(initial_week_scroll_y(now));
            }
        }
    });
    let calendar_for_date = calendar_window.as_weak();
    let calendar_state_for_date = calendar_state.clone();
    let settings_for_calendar_date = shared_settings.clone();
    calendar_window.on_request_select_date(move |date_id| {
        if let Some(calendar) = calendar_for_date.upgrade() {
            calendar_state_for_date
                .borrow_mut()
                .select_date(date_id.as_str());
            refresh_calendar_from_settings(
                &calendar,
                &calendar_state_for_date.borrow(),
                &settings_for_calendar_date.borrow(),
            );
        }
    });
    let calendar_for_month = calendar_window.as_weak();
    let calendar_state_for_month = calendar_state.clone();
    let settings_for_calendar_month = shared_settings.clone();
    calendar_window.on_request_select_month(move |month| {
        if let Some(calendar) = calendar_for_month.upgrade() {
            calendar_state_for_month.borrow_mut().select_month(month);
            refresh_calendar_from_settings(
                &calendar,
                &calendar_state_for_month.borrow(),
                &settings_for_calendar_month.borrow(),
            );
        }
    });
    let calendar_for_scroll = calendar_window.as_weak();
    let calendar_state_for_scroll = calendar_state.clone();
    let settings_for_calendar_scroll = shared_settings.clone();
    calendar_window.on_request_week_scroll(move |y| {
        if let Some(calendar) = calendar_for_scroll.upgrade() {
            let (adjusted, shifted) =
                normalize_week_scroll(&mut calendar_state_for_scroll.borrow_mut(), y);
            if shifted {
                refresh_calendar_from_settings(
                    &calendar,
                    &calendar_state_for_scroll.borrow(),
                    &settings_for_calendar_scroll.borrow(),
                );
                calendar.set_week_scroll_y(adjusted);
            }
        }
    });
    let menu_for_ruler_toggle = context_menu.as_weak();
    let main_window_for_ruler_toggle = window.as_weak();
    context_menu.on_request_toggle_rulers(move || {
        if let Some(menu) = menu_for_ruler_toggle.upgrade() {
            let _ = menu.hide();
        }
        if let Some(main_window) = main_window_for_ruler_toggle.upgrade() {
            main_window.invoke_request_toggle_rulers();
        }
    });
    let main_window_drag_state = configure_main_window_drag(&window);
    let calendar_for_clock_click = calendar_window.as_weak();
    let main_for_clock_click = window.as_weak();
    let state_for_clock_click = calendar_state.clone();
    let settings_for_clock_click = shared_settings.clone();
    let calendar_refresh_for_clock_click = calendar_refresh_generation.clone();
    let focus_guard_for_click = calendar_recently_lost_focus.clone();
    let drag_state_for_click = main_window_drag_state.clone();
    window.on_request_clock_click(move || {
        if drag_state_for_click.take_moved() {
            return;
        }
        if focus_guard_for_click.replace(false) {
            return;
        }
        if let Some(calendar) = calendar_for_clock_click.upgrade() {
            if calendar.window().is_visible() {
                let _ = calendar.hide();
                calendar_refresh_for_clock_click
                    .set(calendar_refresh_for_clock_click.get().wrapping_add(1));
            } else {
                open_calendar_window(
                    &calendar,
                    &main_for_clock_click,
                    &state_for_clock_click,
                    &settings_for_clock_click,
                    &calendar_refresh_for_clock_click,
                );
            }
        }
    });

    let about_window = AboutWindow::new()?;
    about_window.set_version(env!("CARGO_PKG_VERSION").into());
    about_window.set_corner_radius(shared_settings.borrow().corner_radius.clamp(0.0, 15.5) as f32);
    *about_window_for_radius.borrow_mut() = Some(about_window.as_weak());
    let about_for_keyboard = about_window.as_weak();
    about_window
        .window()
        .on_winit_window_event(move |_, event| {
            if is_escape_key(event)
                || is_enter_key(event)
                || matches!(event, WindowEvent::CloseRequested)
                || matches!(event, WindowEvent::Focused(false))
            {
                if let Some(about_window) = about_for_keyboard.upgrade() {
                    about_window.invoke_request_close();
                }
                return EventResult::PreventDefault;
            }
            EventResult::Propagate
        });
    let weak_about_window = about_window.as_weak();
    let main_window_for_about = window.as_weak();
    window.on_request_open_about(move || {
        if let Some(about_window) = weak_about_window.upgrade() {
            open_about_window(&about_window, &main_window_for_about);
        }
    });
    let weak_about_window = about_window.as_weak();
    let main_window_for_about_close = window.as_weak();
    about_window.on_request_close(move || {
        if let Some(about_window) = weak_about_window.upgrade() {
            let _ = about_window.hide();
        }
        set_main_window_modal(&main_window_for_about_close, false);
    });
    about_window.on_request_open_project_url(|| {
        let _ = webbrowser::open("https://github.com/ozinka/OziClock");
    });

    let weak_context_menu = context_menu.as_weak();
    let weak_settings_window = settings_window.as_weak();
    let main_window_for_context_settings = window.as_weak();
    let settings_for_context_open = shared_settings.clone();
    context_menu.on_request_open_settings(move || {
        if let Some(context_menu) = weak_context_menu.upgrade() {
            let _ = context_menu.hide();
        }
        if let Some(settings_window) = weak_settings_window.upgrade() {
            open_settings_window(
                &settings_window,
                &main_window_for_context_settings,
                &settings_for_context_open.borrow(),
            );
        }
    });
    let weak_context_menu = context_menu.as_weak();
    let weak_about_window = about_window.as_weak();
    let main_window_for_context_about = window.as_weak();
    context_menu.on_request_open_about(move || {
        if let Some(context_menu) = weak_context_menu.upgrade() {
            let _ = context_menu.hide();
        }
        if let Some(about_window) = weak_about_window.upgrade() {
            open_about_window(&about_window, &main_window_for_context_about);
        }
    });
    let exit_window = window.as_weak();
    let exit_settings_window = settings_window.as_weak();
    let exit_settings = shared_settings.clone();
    context_menu.on_request_exit(move || {
        save_state_before_exit(&exit_window, &exit_settings_window, &exit_settings);
        let _ = slint::quit_event_loop();
    });
    let exit_window = window.as_weak();
    let exit_settings_window = settings_window.as_weak();
    let exit_settings = shared_settings.clone();
    window.on_request_tray_exit(move || {
        save_state_before_exit(&exit_window, &exit_settings_window, &exit_settings);
        let _ = slint::quit_event_loop();
    });

    let state = shared_settings.clone();
    let weak_window = window.as_weak();
    let explored_time_for_refresh = explored_time.clone();
    window.on_request_refresh_time(move || {
        if explored_time_for_refresh.borrow().is_none()
            && let Some(window) = weak_window.upgrade()
        {
            let state = state.borrow();
            update_clock_tiles(&window, &state.clocks_settings, state.show_seconds);
        }
    });
    schedule_clock_refresh(
        clock_timer.clone(),
        window.as_weak(),
        shared_settings.clone(),
        explored_time.clone(),
        calendar_window.as_weak(),
        calendar_state.clone(),
    );

    let main_window_for_attached_layout = window.as_weak();
    let settings_for_attached_layout = shared_settings.clone();
    let settings_window_for_shutdown = settings_window.as_weak();
    let context_menu_for_shutdown = context_menu.as_weak();
    let about_window_for_shutdown = about_window.as_weak();
    let calendar_window_for_shutdown = calendar_window.as_weak();
    let alarm_attention_for_shutdown = alarm_attention_window.as_weak();
    let timer_attention_for_shutdown = timer_attention_window.as_weak();
    let reminder_attention_for_shutdown = reminder_attention_window.as_weak();
    window.window().on_winit_window_event(move |_, event| {
        if matches!(event, WindowEvent::CloseRequested) {
            save_state_before_exit(
                &main_window_for_attached_layout,
                &settings_window_for_shutdown,
                &settings_for_attached_layout,
            );
            if let Some(settings_window) = settings_window_for_shutdown.upgrade() {
                let _ = settings_window.hide();
            }
            if let Some(context_menu) = context_menu_for_shutdown.upgrade() {
                let _ = context_menu.hide();
            }
            if let Some(about_window) = about_window_for_shutdown.upgrade() {
                let _ = about_window.hide();
            }
            if let Some(calendar_window) = calendar_window_for_shutdown.upgrade() {
                let _ = calendar_window.hide();
            }
            if let Some(alarm_attention) = alarm_attention_for_shutdown.upgrade() {
                let _ = alarm_attention.hide();
            }
            if let Some(timer_attention) = timer_attention_for_shutdown.upgrade() {
                let _ = timer_attention.hide();
            }
            if let Some(reminder_attention) = reminder_attention_for_shutdown.upgrade() {
                let _ = reminder_attention.hide();
            }
            let _ = slint::quit_event_loop();
            return EventResult::PreventDefault;
        }
        EventResult::Propagate
    });

    window.show()?;
    #[cfg(target_os = "macos")]
    window_opacity::sync(&window);
    set_main_window_taskbar_visibility(&window, shared_settings.borrow().show_in_task_bar);
    sync_main_window_size(&window);
    #[cfg(target_os = "windows")]
    let _system_tray = create_system_tray(window.as_weak(), shared_settings.borrow().top_most)?;
    window.run()
}

fn save_state_before_exit(
    window: &slint::Weak<AppWindow>,
    settings_window: &slint::Weak<SettingsWindow>,
    settings: &Rc<RefCell<AppSettings>>,
) {
    if let Some(window) = window.upgrade() {
        persist_main_window_position(&window, &mut settings.borrow_mut());
    }
    if let Some(settings_window) = settings_window.upgrade() {
        persist_settings_window_size(&settings_window, &mut settings.borrow_mut());
    }
    let _ = oziclock_storage::save(&settings.borrow());
}

fn open_about_window(about_window: &AboutWindow, main_window: &slint::Weak<AppWindow>) {
    set_main_window_modal(main_window, true);
    let _ = about_window.show();
    hide_auxiliary_window_from_taskbar(about_window.window());
    position_auxiliary_window_near_clock(about_window.window(), main_window);
    focus_auxiliary_window(about_window.window());
}

fn set_main_window_modal(main_window: &slint::Weak<AppWindow>, modal_open: bool) {
    if let Some(main_window) = main_window.upgrade() {
        main_window.set_modal_open(modal_open);
    }
}

fn persist_main_window_position(window: &AppWindow, settings: &mut AppSettings) {
    let _ = window.window().with_winit_window(|native| {
        if let Ok(position) = native.outer_position() {
            let scale_factor = native.scale_factor();
            settings.main_wnd_left = position.x as f64 / scale_factor;
            settings.main_wnd_top = position.y as f64 / scale_factor;
        }
    });
}

fn set_main_window_taskbar_visibility(window: &AppWindow, show_in_task_bar: bool) {
    #[cfg(target_os = "windows")]
    let _ = window
        .window()
        .with_winit_window(|native| native.set_skip_taskbar(!show_in_task_bar));

    #[cfg(not(target_os = "windows"))]
    let _ = (window, show_in_task_bar);
}

fn apply_clock_scale(window: &AppWindow, clock_scale: f32) {
    window.set_clock_scale(clock_scale);
    sync_main_window_size(window);
}

fn normalize_clock_scale_percent(clock_scale_percent: f32) -> f32 {
    ((clock_scale_percent / 5.0).round() * 5.0).clamp(80.0, 150.0)
}

fn normalize_alert_sound_duration_seconds(seconds: u8) -> u8 {
    match seconds {
        0 | 1 | 5 | 10 | 20 | 30 => seconds,
        _ => 20,
    }
}

fn normalize_border_color(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('#');
    (value.len() == 6 && value.chars().all(|character| character.is_ascii_hexdigit()))
        .then(|| format!("#{}", value.to_ascii_uppercase()))
}

fn initialize_ruler_content(window: &AppWindow, settings: &AppSettings) {
    window.set_label_hours(ModelRc::new(VecModel::from((0..=24).collect::<Vec<i32>>())));
    window.set_rulers(ModelRc::new(VecModel::from(
        settings
            .clocks_settings
            .iter()
            .map(|clock| RulerColumnData {
                accent: parse_color(&clock.color),
            })
            .collect::<Vec<_>>(),
    )));
    let focused_column = settings
        .clocks_settings
        .iter()
        .position(|clock| clock.is_main)
        .unwrap_or(0) as i32;
    window.set_focused_column(focused_column);
    window.set_focus_column_position(focused_column as f32);
    let initial_time_step = initial_ruler_time_step(settings);
    window.set_focus_progress(initial_time_step / 288.0);
    window.set_tick_indices(ModelRc::new(VecModel::from(
        (0..=144).collect::<Vec<i32>>(),
    )));
    window.set_time_step(initial_time_step);
    window.set_hour_labels(ModelRc::new(VecModel::from(slider_hour_labels(
        settings.clocks_settings.len(),
    ))));
}

fn initial_ruler_time_step(settings: &AppSettings) -> f32 {
    let main_zone = settings
        .clocks_settings
        .iter()
        .find(|clock| clock.is_main)
        .or_else(|| settings.clocks_settings.first())
        .and_then(|clock| clock.time_zone.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    let local_now = Utc::now().with_timezone(&main_zone);
    let rounded_hour = (local_now.hour() + u32::from(local_now.minute() >= 30)) % 24;
    (rounded_hour * 12) as f32
}

fn slider_hour_labels(clock_count: usize) -> Vec<i32> {
    match clock_count {
        0 | 1 => vec![0, 12, 24],
        2 => vec![0, 6, 12, 18, 24],
        3 => vec![0, 3, 6, 9, 12, 15, 18, 21, 24],
        _ => (0..=24).step_by(2).collect(),
    }
}

fn format_ruler_label(settings: &AppSettings, column_index: i32, hour: i32) -> String {
    let Some(column) = settings.clocks_settings.get(column_index.max(0) as usize) else {
        return String::new();
    };
    let main_index = settings
        .clocks_settings
        .iter()
        .position(|clock| clock.is_main)
        .unwrap_or(0);
    if column_index as usize == main_index && hour == 24 {
        return "24".into();
    }
    let main_zone = settings.clocks_settings[main_index]
        .time_zone
        .parse::<Tz>()
        .ok();
    let column_zone = column.time_zone.parse::<Tz>().ok();
    let now = Utc::now();
    let offset_hours = match (main_zone, column_zone) {
        (Some(main_zone), Some(column_zone)) => {
            (now.with_timezone(&column_zone)
                .offset()
                .fix()
                .local_minus_utc()
                - now
                    .with_timezone(&main_zone)
                    .offset()
                    .fix()
                    .local_minus_utc()) as f64
                / 3600.0
        }
        _ => 0.0,
    };
    let raw = (hour as f64 + offset_hours).rem_euclid(24.0);
    let whole_hours = raw.floor() as i32;
    let minutes = ((raw - f64::from(whole_hours)) * 60.0).round() as i32;
    if minutes == 0 {
        whole_hours.to_string()
    } else {
        format!("{whole_hours}:{minutes:02}")
    }
}

fn animate_compact_mode(
    window: slint::Weak<AppWindow>,
    compact_mode: bool,
    generation: Rc<Cell<u64>>,
) {
    let revision = generation.get().wrapping_add(1);
    generation.set(revision);
    let start_progress = window
        .upgrade()
        .map(|window| window.get_compact_progress())
        .unwrap_or(if compact_mode { 1.0 } else { 0.0 });
    animate_compact_mode_frame(
        window,
        start_progress,
        if compact_mode { 1.0 } else { 0.0 },
        Instant::now(),
        revision,
        generation,
    );
}

fn animate_compact_mode_frame(
    window: slint::Weak<AppWindow>,
    start_progress: f32,
    target_progress: f32,
    start: Instant,
    revision: u64,
    generation: Rc<Cell<u64>>,
) {
    let progress = (start.elapsed().as_secs_f32() / 0.2).clamp(0.0, 1.0);
    let eased = progress * progress * (3.0 - 2.0 * progress);
    let compact_progress = start_progress + (target_progress - start_progress) * eased;
    if let Some(window) = window.upgrade() {
        window.set_compact_progress(compact_progress);
        set_main_window_height_for_compact_progress(&window, compact_progress);
    }
    if progress < 1.0 {
        Timer::single_shot(Duration::from_millis(16), move || {
            if generation.get() == revision {
                animate_compact_mode_frame(
                    window,
                    start_progress,
                    target_progress,
                    start,
                    revision,
                    generation,
                );
            }
        });
    }
}

fn set_main_window_height_for_compact_progress(window: &AppWindow, compact_progress: f32) {
    let _ = window.window().with_winit_window(|native| {
        let logical_clock_height = 62.0 - 31.0 * compact_progress;
        let logical_height =
            logical_clock_height + if window.get_show_rulers() { 532.0 } else { 0.0 };
        let physical_height =
            (logical_height * window.get_clock_scale() * native.scale_factor() as f32).round()
                as u32;
        let _ = native.request_inner_size(PhysicalSize::new(
            native.inner_size().width,
            physical_height,
        ));
    });
}

fn sync_main_window_size(window: &AppWindow) {
    let _ = window.window().with_winit_window(|native| {
        let system_scale = native.scale_factor() as f32;
        let clock_scale = window.get_clock_scale();
        let logical_width = 1.0 + 100.0 * window.get_clocks().row_count() as f32;
        let clock_height = if window.get_compact_mode() {
            31.0
        } else {
            62.0
        };
        let logical_height = clock_height + if window.get_show_rulers() { 532.0 } else { 0.0 };
        let _ = native.request_inner_size(PhysicalSize::new(
            (logical_width * clock_scale * system_scale).round() as u32,
            (logical_height * clock_scale * system_scale).round() as u32,
        ));
    });
}

fn position_auxiliary_window_near_clock(window: &slint::Window, owner: &slint::Weak<AppWindow>) {
    let Some(owner) = owner.upgrade() else {
        return;
    };

    let _ = owner.window().with_winit_window(|owner_native| {
        let Ok(owner_position) = owner_native.outer_position() else {
            return;
        };
        let Some(work_area) = monitor_work_area(owner_native) else {
            return;
        };
        let _ = window.with_winit_window(|settings_native| {
            let owner_size = owner_native.outer_size();
            let settings_size = settings_native.outer_size();
            let maximum_left = (work_area.right - settings_size.width as i32).max(work_area.left);
            let maximum_top = (work_area.bottom - settings_size.height as i32).max(work_area.top);
            let preferred_left =
                owner_position.x + (owner_size.width as i32 - settings_size.width as i32) / 2;
            let clock_height = if owner.get_compact_mode() { 31.0 } else { 62.0 };
            let below_owner = owner_position.y
                + (clock_height * owner.get_clock_scale() * owner_native.scale_factor() as f32)
                    .round() as i32
                + 8;
            let above_owner = owner_position.y - settings_size.height as i32 - 8;
            let preferred_top = if below_owner <= maximum_top {
                below_owner
            } else {
                above_owner
            };
            settings_native.set_outer_position(PhysicalPosition::new(
                preferred_left.clamp(work_area.left, maximum_left),
                preferred_top.clamp(work_area.top, maximum_top),
            ));
        });
    });
}

fn open_calendar_window(
    calendar: &CalendarWindow,
    owner: &slint::Weak<AppWindow>,
    state: &Rc<RefCell<CalendarState>>,
    settings: &Rc<RefCell<AppSettings>>,
    refresh_generation: &Rc<Cell<u64>>,
) {
    {
        let settings = settings.borrow();
        let mut state = state.borrow_mut();
        state.light_theme = settings.calendar_light_theme;
        state.monday_first = settings.calendar_monday_first;
    }
    let accent = calendar_accent(&settings.borrow());
    calendar.set_accent(accent);
    calendar.set_accent_foreground(accent_foreground(accent));
    calendar.set_corner_radius(settings.borrow().corner_radius.clamp(0.0, 15.5) as f32);
    refresh_calendar_from_settings(calendar, &state.borrow(), &settings.borrow());
    if state.borrow().view == CalendarView::Week {
        calendar.set_week_scroll_y(initial_week_scroll_y(calendar_local_now(
            &settings.borrow(),
        )));
    }
    let _ = calendar.show();
    hide_auxiliary_window_from_taskbar(calendar.window());
    position_calendar_window(calendar.window(), owner);
    focus_auxiliary_window(calendar.window());
    let revision = refresh_generation.get().wrapping_add(1);
    refresh_generation.set(revision);
    schedule_calendar_boundary_refresh(
        calendar.as_weak(),
        state.clone(),
        settings.clone(),
        refresh_generation.clone(),
        revision,
    );
}

fn position_calendar_window(window: &slint::Window, owner: &slint::Weak<AppWindow>) {
    let Some(owner) = owner.upgrade() else {
        return;
    };
    let _ = owner.window().with_winit_window(|owner_native| {
        let Ok(owner_position) = owner_native.outer_position() else {
            return;
        };
        let Some(work_area) = monitor_work_area(owner_native) else {
            return;
        };
        let _ = window.with_winit_window(|calendar| {
            let owner_size = owner_native.outer_size();
            let calendar_size = calendar.outer_size();
            let centered_left =
                owner_position.x + (owner_size.width as i32 - calendar_size.width as i32) / 2;
            let left = centered_left.clamp(
                work_area.left,
                (work_area.right - calendar_size.width as i32).max(work_area.left),
            );
            let below = owner_position.y + owner_size.height as i32;
            let above = owner_position.y - calendar_size.height as i32;
            let top = if below + calendar_size.height as i32 <= work_area.bottom {
                below
            } else {
                above.max(work_area.top)
            };
            calendar.set_outer_position(PhysicalPosition::new(left, top));
        });
    });
}

fn calendar_local_now(settings: &AppSettings) -> NaiveDateTime {
    let zone = settings
        .clocks_settings
        .iter()
        .find(|clock| clock.is_main)
        .and_then(|clock| clock.time_zone.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    Utc::now().with_timezone(&zone).naive_local()
}

fn refresh_calendar_from_settings(
    window: &CalendarWindow,
    state: &CalendarState,
    settings: &AppSettings,
) {
    let now = calendar_local_now(settings);
    let today =
        CalendarDate::new(now.year(), now.month(), now.day()).expect("current local date is valid");
    refresh_calendar_window(window, state, today, now);
}

fn schedule_calendar_boundary_refresh(
    window: slint::Weak<CalendarWindow>,
    state: Rc<RefCell<CalendarState>>,
    settings: Rc<RefCell<AppSettings>>,
    generation: Rc<Cell<u64>>,
    revision: u64,
) {
    let seconds = 60 - Utc::now().second() as u64;
    Timer::single_shot(Duration::from_secs(seconds), move || {
        if generation.get() != revision {
            return;
        }
        if let Some(window) = window.upgrade() {
            let current_settings = settings.borrow();
            let now = calendar_local_now(&current_settings);
            let today = CalendarDate::new(now.year(), now.month(), now.day())
                .expect("current local date is valid");
            let calendar_state = state.borrow();
            refresh_calendar_window(&window, &calendar_state, today, now);
            drop(calendar_state);
            drop(current_settings);
            schedule_calendar_boundary_refresh(
                window.as_weak(),
                state,
                settings,
                generation,
                revision,
            );
        }
    });
}

fn refresh_auxiliary_accents(
    planner: &slint::Weak<PlannerWindow>,
    calendar: &slint::Weak<CalendarWindow>,
    settings: &AppSettings,
) {
    let accent = calendar_accent(settings);
    if let Some(planner) = planner.upgrade() {
        planner.set_accent(accent.brighter(0.4));
    }
    if let Some(calendar) = calendar.upgrade() {
        calendar.set_accent(accent);
        calendar.set_accent_foreground(accent_foreground(accent));
    }
}

fn calendar_accent(settings: &AppSettings) -> slint::Color {
    clock_accent(&settings.clocks_settings)
}

fn clock_accent(clocks: &[ClockSettings]) -> slint::Color {
    let clock = clocks
        .iter()
        .find(|clock| clock.is_main)
        .or_else(|| clocks.first());
    let Some(clock) = clock else {
        return slint::Color::from_rgb_u8(79, 117, 117);
    };
    let (hue, saturation, _) = color_to_hsv(&clock.color);
    hsv_color(hue, saturation, 50.0)
}

fn refresh_task_models(
    planner: &PlannerWindow,
    settings: &AppSettings,
    open_model: &Rc<VecModel<PlannerTaskData>>,
    completed_model: &Rc<VecModel<PlannerTaskData>>,
) {
    open_model.set_vec(planner_task_rows(settings));
    completed_model.set_vec(planner_completed_task_rows(settings));
    planner.set_task_count(open_task_count(settings));
    planner.set_completed_task_count(completed_task_count(settings));
}

fn apply_task_command(
    settings: &Rc<RefCell<AppSettings>>,
    planner: &slint::Weak<PlannerWindow>,
    open_model: &Rc<VecModel<PlannerTaskData>>,
    completed_model: &Rc<VecModel<PlannerTaskData>>,
    command: PlannerCommand,
) {
    let mut settings = settings.borrow_mut();
    let mut updated = settings.clone();
    if !execute_planner_command(&mut updated.planner, command)
        || oziclock_storage::save(&updated).is_err()
    {
        return;
    }
    *settings = updated;
    if let Some(planner) = planner.upgrade() {
        refresh_task_models(&planner, &settings, open_model, completed_model);
        planner.set_selected_task("".into());
        planner.set_editing_task("".into());
        planner.set_task_draft("".into());
        planner.set_task_error("".into());
    }
}

fn main_time_zone(settings: &AppSettings) -> String {
    settings
        .clocks_settings
        .iter()
        .find(|clock| clock.is_main)
        .or_else(|| settings.clocks_settings.first())
        .map(|clock| clock.time_zone.clone())
        .unwrap_or_else(|| "UTC".to_owned())
}

fn default_reminder_datetime(settings: &AppSettings) -> (String, String) {
    let zone = main_time_zone(settings)
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC);
    let local = (Utc::now() + chrono::Duration::minutes(10)).with_timezone(&zone);
    (
        local.format("%Y-%m-%d").to_string(),
        local.format("%H:%M").to_string(),
    )
}

fn refresh_reminder_calendar(
    planner: &PlannerWindow,
    model: &VecModel<ReminderCalendarDayData>,
    anchor: NaiveDate,
    selected: Option<NaiveDate>,
    today: NaiveDate,
) {
    let first = anchor
        .with_day(1)
        .expect("the first day exists in every month");
    let grid_start = first - chrono::Duration::days(first.weekday().num_days_from_monday().into());
    let days: Vec<_> = (0..42)
        .map(|offset| {
            let date = grid_start + chrono::Duration::days(offset);
            ReminderCalendarDayData {
                label: date.day().to_string().into(),
                date: date.format("%Y-%m-%d").to_string().into(),
                in_month: date.month() == anchor.month() && date.year() == anchor.year(),
                enabled: date >= today,
                selected: selected == Some(date),
                today: date == today,
            }
        })
        .collect();
    model.set_vec(days);
    planner.set_reminder_calendar_title(anchor.format("%B %Y").to_string().into());
}

fn current_week_start(settings: &AppSettings) -> NaiveDate {
    let today = calendar_local_now(settings).date();
    today - chrono::Duration::days(today.weekday().num_days_from_monday().into())
}

fn refresh_plan_overview(
    planner: &PlannerWindow,
    month_model: &VecModel<PlanMonthDayData>,
    year_model: &VecModel<PlanYearMonthData>,
    settings: &AppSettings,
    anchor: NaiveDate,
) {
    let today = calendar_local_now(settings).date();
    let first = anchor.with_day(1).expect("month has a first day");
    let grid_start = first - chrono::Duration::days(first.weekday().num_days_from_monday().into());
    month_model.set_vec(
        (0..42)
            .map(|offset| {
                let date = grid_start + chrono::Duration::days(offset);
                let (event_count, task_count) = plan_counts_for_date(settings, date);
                PlanMonthDayData {
                    label: date.day().to_string().into(),
                    date: date.format("%Y-%m-%d").to_string().into(),
                    in_month: date.month() == anchor.month(),
                    today: date == today,
                    event_count,
                    task_count,
                }
            })
            .collect::<Vec<_>>(),
    );
    year_model.set_vec(
        (1..=12)
            .map(|month| {
                let mut event_count = 0;
                let mut task_count = 0;
                let mut date =
                    NaiveDate::from_ymd_opt(anchor.year(), month, 1).expect("valid month");
                while date.month() == month {
                    let counts = plan_counts_for_date(settings, date);
                    event_count += counts.0;
                    task_count += counts.1;
                    date += chrono::Duration::days(1);
                }
                PlanYearMonthData {
                    label: NaiveDate::from_ymd_opt(anchor.year(), month, 1)
                        .unwrap()
                        .format("%B")
                        .to_string()
                        .into(),
                    month: month as i32,
                    current: today.year() == anchor.year() && today.month() == month,
                    event_count,
                    task_count,
                }
            })
            .collect::<Vec<_>>(),
    );
    planner.set_plan_week_title(
        if planner.get_selected_view() == 1 {
            anchor.format("%B %Y")
        } else {
            anchor.format("%Y")
        }
        .to_string()
        .into(),
    );
}

fn refresh_plan_week(
    planner: &PlannerWindow,
    day_model: &VecModel<PlanWeekDayData>,
    task_model: &VecModel<PlanTaskMarkerData>,
    reminder_model: &VecModel<PlanReminderMarkerData>,
    event_models: (
        &VecModel<PlanEventMarkerData>,
        &VecModel<PlanAllDayEventData>,
    ),
    settings: &AppSettings,
    start: NaiveDate,
) {
    let local_now = calendar_local_now(settings);
    let today = local_now.date();
    day_model.set_vec(
        (0..7)
            .map(|offset| {
                let date = start + chrono::Duration::days(offset);
                PlanWeekDayData {
                    label: date.format("%a %-d").to_string().into(),
                    date: date.format("%Y-%m-%d").to_string().into(),
                    weekend: date.weekday().num_days_from_monday() >= 5,
                    today: date == today,
                }
            })
            .collect::<Vec<_>>(),
    );
    let reminder_rows = plan_reminder_rows(settings, start);
    let selected_reminder = planner.get_selected_plan_reminder();
    if !selected_reminder.is_empty()
        && !reminder_rows
            .iter()
            .any(|row| row.id.as_str() == selected_reminder.as_str())
    {
        planner.set_selected_plan_date("".into());
        planner.set_selected_plan_hour(-1);
        planner.set_selected_plan_reminder("".into());
        planner.set_selected_plan_title("".into());
        planner.set_selected_plan_time("".into());
    }
    reminder_model.set_vec(reminder_rows);
    task_model.set_vec(plan_task_rows(settings, start));
    let (event_rows, all_day_rows) = plan_event_rows(settings, start);
    let selected_event = planner.get_selected_plan_event();
    if !selected_event.is_empty()
        && !event_rows
            .iter()
            .any(|row| row.id.as_str() == selected_event.as_str())
        && !all_day_rows
            .iter()
            .any(|row| row.id.as_str() == selected_event.as_str())
    {
        planner.set_selected_plan_date("".into());
        planner.set_selected_plan_event("".into());
        planner.set_selected_plan_title("".into());
        planner.set_selected_plan_time("".into());
    }
    event_models.0.set_vec(event_rows);
    event_models.1.set_vec(all_day_rows);
    let current_day = today.signed_duration_since(start).num_days();
    let current_minute =
        i32::try_from(local_now.time().num_seconds_from_midnight() / 60).unwrap_or_default();
    if (0..7).contains(&current_day) {
        planner.set_current_plan_day(current_day as i32);
        planner.set_current_plan_minute(current_minute);
        planner.set_current_plan_time(local_now.time().format("%H:%M").to_string().into());
    } else {
        planner.set_current_plan_day(-1);
        planner.set_current_plan_minute(-1);
        planner.set_current_plan_time("".into());
    }
    let title = if start == current_week_start(settings) {
        "This week".to_owned()
    } else {
        format!(
            "{} – {}",
            start.format("%-d %b"),
            (start + chrono::Duration::days(6)).format("%-d %b")
        )
    };
    planner.set_plan_week_title(title.into());
}

#[derive(Clone)]
struct ReminderAttentionItem {
    reminder_id: PlannerId,
    title: String,
}

#[derive(Clone)]
struct EventAttentionItem {
    event_id: PlannerId,
    occurrence_utc: String,
    title: String,
    time: String,
}

#[derive(Clone)]
struct TaskAttentionItem {
    task_id: PlannerId,
    title: String,
    time: String,
}

fn task_attention_item(id: &PlannerId, settings: &AppSettings) -> Option<TaskAttentionItem> {
    let task = settings.planner.tasks.iter().find(|task| &task.id == id)?;
    let due = task
        .attention_due_utc
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())?;
    let zone = main_time_zone(settings)
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC);
    Some(TaskAttentionItem {
        task_id: id.clone(),
        title: task.title.clone(),
        time: due
            .with_timezone(&zone)
            .format("Due %d %b · %H:%M")
            .to_string(),
    })
}

fn pending_task_attention_items(settings: &AppSettings) -> VecDeque<TaskAttentionItem> {
    oziclock_app::task_time::pending_attention(&settings.planner)
        .iter()
        .filter_map(|id| task_attention_item(id, settings))
        .collect()
}

fn display_task_attention(
    window: &slint::Weak<TaskAttentionWindow>,
    owner: &slint::Weak<AppWindow>,
    queue: &Rc<RefCell<VecDeque<TaskAttentionItem>>>,
) {
    let Some(window) = window.upgrade() else {
        return;
    };
    let Some(item) = queue.borrow().front().cloned() else {
        let _ = window.hide();
        return;
    };
    window.set_task_title(item.title.into());
    window.set_task_time(item.time.into());
    if window.show().is_ok() {
        hide_auxiliary_window_from_taskbar(window.window());
        position_calendar_window(window.window(), owner);
    }
}

fn schedule_task_attention_pulse(timer: Rc<Timer>, window: slint::Weak<TaskAttentionWindow>) {
    let next_timer = timer.clone();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(450),
        move || {
            if let Some(window) = window.upgrade() {
                window.set_pulse(!window.get_pulse());
            }
            schedule_task_attention_pulse(next_timer.clone(), window.clone());
        },
    );
}

fn event_attention_item(
    receipt: &EventReceipt,
    settings: &AppSettings,
) -> Option<EventAttentionItem> {
    let event = settings
        .planner
        .events
        .iter()
        .find(|event| event.id == receipt.event_id)?;
    let occurrence = DateTime::parse_from_rfc3339(&receipt.occurrence_utc).ok()?;
    let zone = main_time_zone(settings)
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC);
    Some(EventAttentionItem {
        event_id: receipt.event_id.clone(),
        occurrence_utc: receipt.occurrence_utc.clone(),
        title: event.title.clone(),
        time: occurrence
            .with_timezone(&zone)
            .format("Starts %d %b · %H:%M")
            .to_string(),
    })
}

fn pending_event_attention_items(settings: &AppSettings) -> VecDeque<EventAttentionItem> {
    oziclock_app::event_time::pending_attention(&settings.planner)
        .iter()
        .filter_map(|receipt| event_attention_item(receipt, settings))
        .collect()
}

fn display_event_attention(
    window: &slint::Weak<EventAttentionWindow>,
    owner: &slint::Weak<AppWindow>,
    queue: &Rc<RefCell<VecDeque<EventAttentionItem>>>,
) {
    let Some(window) = window.upgrade() else {
        return;
    };
    let Some(item) = queue.borrow().front().cloned() else {
        let _ = window.hide();
        return;
    };
    window.set_event_title(item.title.into());
    window.set_event_time(item.time.into());
    if window.show().is_ok() {
        hide_auxiliary_window_from_taskbar(window.window());
        position_calendar_window(window.window(), owner);
    }
}

fn schedule_event_attention_pulse(timer: Rc<Timer>, window: slint::Weak<EventAttentionWindow>) {
    let next_timer = timer.clone();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(450),
        move || {
            if let Some(window) = window.upgrade() {
                window.set_pulse(!window.get_pulse());
            }
            schedule_event_attention_pulse(next_timer.clone(), window.clone());
        },
    );
}

fn pending_reminder_attention_items(settings: &AppSettings) -> VecDeque<ReminderAttentionItem> {
    oziclock_app::reminder_time::pending_attention(&settings.planner)
        .into_iter()
        .filter_map(|id| {
            settings
                .planner
                .reminders
                .iter()
                .find(|reminder| reminder.id == id)
                .map(|reminder| ReminderAttentionItem {
                    reminder_id: id,
                    title: reminder.title.clone(),
                })
        })
        .collect()
}

fn display_reminder_attention(
    window: &slint::Weak<ReminderAttentionWindow>,
    owner: &slint::Weak<AppWindow>,
    queue: &Rc<RefCell<VecDeque<ReminderAttentionItem>>>,
) {
    let Some(window) = window.upgrade() else {
        return;
    };
    let Some(item) = queue.borrow().front().cloned() else {
        let _ = window.hide();
        return;
    };
    window.set_reminder_title(item.title.into());
    if window.show().is_ok() {
        hide_auxiliary_window_from_taskbar(window.window());
        position_calendar_window(window.window(), owner);
    }
}

fn schedule_reminder_attention_pulse(
    timer: Rc<Timer>,
    window: slint::Weak<ReminderAttentionWindow>,
) {
    let next_timer = timer.clone();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(450),
        move || {
            if let Some(window) = window.upgrade() {
                window.set_pulse(!window.get_pulse());
            }
            schedule_reminder_attention_pulse(next_timer.clone(), window.clone());
        },
    );
}

#[derive(Clone)]
struct PlanRefreshModels {
    days: Rc<VecModel<PlanWeekDayData>>,
    tasks: Rc<VecModel<PlanTaskMarkerData>>,
    month_days: Rc<VecModel<PlanMonthDayData>>,
    year_months: Rc<VecModel<PlanYearMonthData>>,
    reminders: Rc<VecModel<PlanReminderMarkerData>>,
    events: Rc<VecModel<PlanEventMarkerData>>,
    all_day_events: Rc<VecModel<PlanAllDayEventData>>,
}

fn schedule_plan_refresh(
    timer: Rc<Timer>,
    planner: slint::Weak<PlannerWindow>,
    models: PlanRefreshModels,
    settings: Rc<RefCell<AppSettings>>,
    start: Rc<RefCell<NaiveDate>>,
) {
    let next_timer = timer.clone();
    let next_models = models.clone();
    let next_settings = settings.clone();
    let next_start = start.clone();
    timer.start(TimerMode::SingleShot, Duration::from_secs(60), move || {
        if let Some(planner) = planner.upgrade() {
            if planner.get_selected_view() == 0 {
                refresh_plan_week(
                    &planner,
                    &models.days,
                    &models.tasks,
                    &models.reminders,
                    (&models.events, &models.all_day_events),
                    &settings.borrow(),
                    *start.borrow(),
                );
            } else {
                refresh_plan_overview(
                    &planner,
                    &models.month_days,
                    &models.year_months,
                    &settings.borrow(),
                    *start.borrow(),
                );
            }
        }
        schedule_plan_refresh(
            next_timer.clone(),
            planner.clone(),
            next_models.clone(),
            next_settings.clone(),
            next_start.clone(),
        );
    });
}

fn schedule_event_refresh(
    alert_sound: alert_sound::AlertSound,
    timer: Rc<Timer>,
    settings: Rc<RefCell<AppSettings>>,
    attention: slint::Weak<EventAttentionWindow>,
    owner: slint::Weak<AppWindow>,
    queue: Rc<RefCell<VecDeque<EventAttentionItem>>>,
) {
    let next_timer = timer.clone();
    let next_settings = settings.clone();
    timer.start(TimerMode::SingleShot, Duration::from_secs(1), move || {
        let now = Utc::now();
        let mut settings = next_settings.borrow_mut();
        let checked = settings
            .planner
            .event_checked_at_utc
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or(now - chrono::Duration::seconds(1));
        let after = checked.max(now - chrono::Duration::minutes(5));
        let zone = main_time_zone(&settings);
        let mut updated = settings.clone();
        let receipts = oziclock_app::event_time::reconcile(&mut updated.planner, after, now, &zone);
        oziclock_storage::prune_event_receipts(&mut updated, now);
        if oziclock_storage::save(&updated).is_ok() {
            *settings = updated;
            let was_empty = queue.borrow().is_empty();
            for receipt in &receipts {
                if let Some(item) = event_attention_item(receipt, &settings) {
                    delivery_feedback::deliver(
                        &item.title,
                        &item.time,
                        settings.alert_sound_duration_seconds,
                        &alert_sound,
                    );
                    queue.borrow_mut().push_back(item);
                }
            }
            if was_empty && !queue.borrow().is_empty() {
                display_event_attention(&attention, &owner, &queue);
            }
        }
        drop(settings);
        schedule_event_refresh(
            alert_sound.clone(),
            next_timer.clone(),
            next_settings.clone(),
            attention.clone(),
            owner.clone(),
            queue.clone(),
        );
    });
}

fn schedule_task_refresh(
    alert_sound: alert_sound::AlertSound,
    timer: Rc<Timer>,
    settings: Rc<RefCell<AppSettings>>,
    attention: slint::Weak<TaskAttentionWindow>,
    owner: slint::Weak<AppWindow>,
    queue: Rc<RefCell<VecDeque<TaskAttentionItem>>>,
) {
    let next_timer = timer.clone();
    let next_settings = settings.clone();
    timer.start(TimerMode::SingleShot, Duration::from_secs(1), move || {
        let mut settings = next_settings.borrow_mut();
        let mut updated = settings.clone();
        let delivered = oziclock_app::task_time::reconcile(&mut updated.planner, Utc::now());
        if !delivered.is_empty() && oziclock_storage::save(&updated).is_ok() {
            *settings = updated;
            let was_empty = queue.borrow().is_empty();
            for id in &delivered {
                if let Some(item) = task_attention_item(id, &settings) {
                    delivery_feedback::deliver(
                        &item.title,
                        &item.time,
                        settings.alert_sound_duration_seconds,
                        &alert_sound,
                    );
                    queue.borrow_mut().push_back(item);
                }
            }
            if was_empty {
                display_task_attention(&attention, &owner, &queue);
            }
        }
        drop(settings);
        schedule_task_refresh(
            alert_sound.clone(),
            next_timer.clone(),
            next_settings.clone(),
            attention.clone(),
            owner.clone(),
            queue.clone(),
        );
    });
}

fn schedule_reminder_refresh(
    alert_sound: alert_sound::AlertSound,
    timer: Rc<Timer>,
    model: Rc<VecModel<PlannerReminderData>>,
    settings: Rc<RefCell<AppSettings>>,
    attention: slint::Weak<ReminderAttentionWindow>,
    owner: slint::Weak<AppWindow>,
    queue: Rc<RefCell<VecDeque<ReminderAttentionItem>>>,
) {
    let next_timer = timer.clone();
    let next_model = model.clone();
    let next_settings = settings.clone();
    timer.start(TimerMode::SingleShot, Duration::from_secs(1), move || {
        let mut settings = next_settings.borrow_mut();
        let mut updated = settings.clone();
        let delivered = oziclock_app::reminder_time::reconcile(&mut updated.planner, Utc::now());
        if !delivered.is_empty() && oziclock_storage::save(&updated).is_ok() {
            *settings = updated;
            let was_empty = queue.borrow().is_empty();
            for id in delivered {
                if let Some(reminder) = settings
                    .planner
                    .reminders
                    .iter()
                    .find(|reminder| reminder.id == id)
                {
                    delivery_feedback::deliver(
                        &reminder.title,
                        "Reminder is due",
                        settings.alert_sound_duration_seconds,
                        &alert_sound,
                    );
                    queue.borrow_mut().push_back(ReminderAttentionItem {
                        reminder_id: id,
                        title: reminder.title.clone(),
                    });
                }
            }
            if was_empty && !queue.borrow().is_empty() {
                display_reminder_attention(&attention, &owner, &queue);
            }
        }
        next_model.set_vec(planner_reminder_rows(&settings));
        drop(settings);
        schedule_reminder_refresh(
            alert_sound.clone(),
            next_timer.clone(),
            next_model.clone(),
            next_settings.clone(),
            attention.clone(),
            owner.clone(),
            queue.clone(),
        );
    });
}

fn stopwatch_elapsed_milliseconds(settings: &AppSettings, started_at: Option<Instant>) -> u128 {
    settings.planner.stopwatch.as_ref().map_or(0, |stopwatch| {
        u128::from(stopwatch.elapsed_seconds) * 1_000 + u128::from(stopwatch.elapsed_milliseconds)
    }) + started_at
        .map(|started_at| started_at.elapsed().as_millis())
        .unwrap_or(0)
}

fn refresh_stopwatch_ui(
    planner: &PlannerWindow,
    settings: &AppSettings,
    started_at: Option<Instant>,
    lap_model: &VecModel<StopwatchLapData>,
) {
    let (time, milliseconds) =
        format_stopwatch(stopwatch_elapsed_milliseconds(settings, started_at));
    planner.set_stopwatch_time(time.into());
    planner.set_stopwatch_milliseconds(milliseconds.into());
    planner.set_stopwatch_running(
        settings
            .planner
            .stopwatch
            .as_ref()
            .is_some_and(|stopwatch| stopwatch.state == StopwatchState::Running),
    );
    planner.set_stopwatch_has_data(
        stopwatch_elapsed_milliseconds(settings, started_at) > 0
            || !stopwatch_lap_rows(settings).is_empty(),
    );
    lap_model.set_vec(stopwatch_lap_rows(settings));
}

fn schedule_stopwatch_refresh(
    timer: Rc<Timer>,
    planner: slint::Weak<PlannerWindow>,
    settings: Rc<RefCell<AppSettings>>,
    started_at: Rc<Cell<Option<Instant>>>,
    lap_model: Rc<VecModel<StopwatchLapData>>,
) {
    let next_timer = timer.clone();
    let next_planner = planner.clone();
    let next_settings = settings.clone();
    let next_started_at = started_at.clone();
    let next_lap_model = lap_model.clone();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(16),
        move || {
            if let Some(planner) = next_planner.upgrade() {
                let settings = next_settings.borrow();
                if settings
                    .planner
                    .stopwatch
                    .as_ref()
                    .is_some_and(|stopwatch| stopwatch.state == StopwatchState::Running)
                {
                    refresh_stopwatch_ui(
                        &planner,
                        &settings,
                        next_started_at.get(),
                        &next_lap_model,
                    );
                }
            }
            schedule_stopwatch_refresh(
                next_timer.clone(),
                next_planner.clone(),
                next_settings.clone(),
                next_started_at.clone(),
                next_lap_model.clone(),
            );
        },
    );
}

fn accent_foreground(accent: slint::Color) -> slint::Color {
    let luminance = 0.2126 * f64::from(accent.red())
        + 0.7152 * f64::from(accent.green())
        + 0.0722 * f64::from(accent.blue());
    if luminance > 150.0 {
        slint::Color::from_rgb_u8(23, 32, 39)
    } else {
        slint::Color::from_rgb_u8(255, 255, 255)
    }
}

struct WorkArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(target_os = "windows")]
fn monitor_work_area(window: &slint::winit_030::winit::window::Window) -> Option<WorkArea> {
    let monitor = window.current_monitor()?;
    monitor_work_area_for_handle(monitor.hmonitor() as _)
}

#[cfg(target_os = "windows")]
fn monitor_work_area_for_handle(monitor: *mut std::ffi::c_void) -> Option<WorkArea> {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let success = unsafe { GetMonitorInfoW(monitor, &mut info) };
    if success == 0 {
        return None;
    }
    Some(WorkArea {
        left: info.rcWork.left,
        top: info.rcWork.top,
        right: info.rcWork.right,
        bottom: info.rcWork.bottom,
    })
}

#[cfg(not(target_os = "windows"))]
fn monitor_work_area(window: &slint::winit_030::winit::window::Window) -> Option<WorkArea> {
    let monitor = window.current_monitor()?;
    let position = monitor.position();
    let size = monitor.size();
    Some(WorkArea {
        left: position.x,
        top: position.y,
        right: position.x + size.width as i32,
        bottom: position.y + size.height as i32,
    })
}

fn update_clock_tiles(window: &AppWindow, settings: &[ClockSettings], show_seconds: bool) {
    update_clock_tiles_at(window, settings, show_seconds, Utc::now());
}

fn update_clock_tiles_at(
    window: &AppWindow,
    settings: &[ClockSettings],
    show_seconds: bool,
    now: DateTime<Utc>,
) {
    let clocks: Vec<ClockTileData> = settings
        .iter()
        .map(|settings| to_clock_tile(settings, now, show_seconds))
        .collect();

    window.set_clocks(ModelRc::new(VecModel::from(clocks)));
}

fn ruler_time_step_to_utc(settings: &AppSettings, time_step: f32) -> DateTime<Utc> {
    let main_zone = settings
        .clocks_settings
        .iter()
        .find(|clock| clock.is_main)
        .or_else(|| settings.clocks_settings.first())
        .and_then(|clock| clock.time_zone.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    let local_now = Utc::now().with_timezone(&main_zone);
    let selected_minutes = (time_step.round() as u32).clamp(0, 288) * 5;
    let selected_local =
        local_now
            .date_naive()
            .and_hms_opt(selected_minutes / 60, selected_minutes % 60, 0);
    selected_local
        .and_then(
            |selected_local| match main_zone.from_local_datetime(&selected_local) {
                LocalResult::Single(value) => Some(value.with_timezone(&Utc)),
                LocalResult::Ambiguous(value, _) => Some(value.with_timezone(&Utc)),
                LocalResult::None => None,
            },
        )
        .unwrap_or_else(Utc::now)
}

fn refresh_clock_order(main_window: &slint::Weak<AppWindow>, settings: &AppSettings) {
    if let Some(main_window) = main_window.upgrade() {
        update_clock_tiles(
            &main_window,
            &settings.clocks_settings,
            settings.show_seconds,
        );
        initialize_ruler_content(&main_window, settings);
        sync_main_window_size(&main_window);
    }
}

fn show_context_menu(context_menu: &ContextMenuWindow, owner: &AppWindow) {
    context_menu.set_show_rulers(owner.get_show_rulers());
    let _ = context_menu.show();
    hide_auxiliary_window_from_taskbar(context_menu.window());
    let requested_x = owner.get_menu_x();
    let _ = owner.window().with_winit_window(|winit_owner| {
        let owner_position = winit_owner.outer_position().unwrap_or_default();
        let scale_factor = winit_owner.scale_factor();
        let Some(work_area) = monitor_work_area(winit_owner) else {
            return;
        };
        let _ = context_menu.window().with_winit_window(|menu| {
            let menu_size = menu.outer_size();
            let requested_left = owner_position.x + (requested_x * scale_factor as f32) as i32;
            let maximum_left = work_area.right - menu_size.width as i32;
            let left = requested_left.clamp(work_area.left, maximum_left);
            let clock_height = if owner.get_compact_mode() { 31.0 } else { 62.0 };
            let below = owner_position.y
                + (clock_height * owner.get_clock_scale() * scale_factor as f32).round() as i32;
            let above = owner_position.y - menu_size.height as i32;
            let maximum_top = work_area.bottom - menu_size.height as i32;
            let top = if below <= maximum_top {
                below
            } else {
                above.clamp(work_area.top, maximum_top)
            };
            menu.set_outer_position(PhysicalPosition::new(left, top));
        });
    });
    focus_auxiliary_window(context_menu.window());
}

fn focus_auxiliary_window(window: &slint::Window) {
    let _ = window.with_winit_window(|window| window.focus_window());
}

fn is_escape_key(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::KeyboardInput { event, .. }
            if event.state == ElementState::Pressed
                && matches!(event.logical_key, Key::Named(NamedKey::Escape))
    )
}

fn is_enter_key(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::KeyboardInput { event, .. }
            if event.state == ElementState::Pressed
                && matches!(event.logical_key, Key::Named(NamedKey::Enter))
    )
}

#[cfg(target_os = "windows")]
fn hide_auxiliary_window_from_taskbar(window: &slint::Window) {
    let _ = window.with_winit_window(|window| window.set_skip_taskbar(true));
}

#[cfg(not(target_os = "windows"))]
fn hide_auxiliary_window_from_taskbar(_window: &slint::Window) {}

fn to_clock_tile(
    settings: &ClockSettings,
    now: DateTime<Utc>,
    show_seconds: bool,
) -> ClockTileData {
    let timezone = settings.time_zone.parse::<Tz>().unwrap_or(chrono_tz::UTC);
    let local_time = now.with_timezone(&timezone);
    ClockTileData {
        label: settings.label.clone().into(),
        month: local_time.format("%m/").to_string().into(),
        day: local_time.format("%d").to_string().into(),
        hour: local_time.format("%H").to_string().into(),
        minute: local_time.format("%M").to_string().into(),
        second: if show_seconds {
            local_time.format("%S").to_string().into()
        } else {
            "".into()
        },
        accent: parse_color(&settings.color),
        main_zone: settings.is_main,
    }
}

#[cfg(test)]
mod timer_editor_tests {
    use super::*;

    #[test]
    fn accent_tracks_color_edits_and_main_clock_selection() {
        let mut clocks = vec![
            ClockSettings {
                label: "A".into(),
                time_zone: "UTC".into(),
                color: "#FF0000".into(),
                is_main: true,
            },
            ClockSettings {
                label: "B".into(),
                time_zone: "UTC".into(),
                color: "#0000FF".into(),
                is_main: false,
            },
        ];
        let original = clock_accent(&clocks);
        clocks[0].color = "#00FF00".into();
        let edited = clock_accent(&clocks);
        assert_ne!(original, edited);
        clocks[0].is_main = false;
        clocks[1].is_main = true;
        assert_ne!(edited, clock_accent(&clocks));
        assert_eq!(clock_accent(&clocks), clock_accent(&clocks[1..]));
        clocks[1].is_main = false;
        assert_eq!(clock_accent(&clocks), edited);
    }
}
