use std::{cell::RefCell, collections::VecDeque, rc::Rc, time::Duration};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use oziclock_app::planner::{PlannerCommand, execute_planner_command};
use oziclock_storage::{AppSettings, PlannerId, PlannerTimer, TimerState};
use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel};

use super::{
    AppWindow, PlannerTimerData, PlannerWindow, TimerAttentionWindow,
    alert_sound::AlertSound,
    delivery_feedback, hide_auxiliary_window_from_taskbar,
    planner_inputs::{
        adjust_timer_part, duration_parts, mask_timer_part, parse_timer_duration, parse_timer_part,
        store_timer_part,
    },
    planner_models::{planner_timer_rows, timer_display_title},
    position_calendar_window,
};

pub(super) fn wire_timer_bindings(
    alert_sound: AlertSound,
    planner_window: &PlannerWindow,
    attention_window: &TimerAttentionWindow,
    owner: &AppWindow,
    shared_settings: Rc<RefCell<AppSettings>>,
) {
    let timer_model = Rc::new(VecModel::from(planner_timer_rows(
        &shared_settings.borrow(),
    )));
    planner_window.set_timers(ModelRc::from(timer_model.clone()));
    let attention_queue = Rc::new(RefCell::new(pending_timer_attention_items(
        &shared_settings.borrow(),
    )));

    let queue_for_timer_dismiss = attention_queue.clone();
    let sound_for_timer_dismiss = alert_sound.clone();
    let settings_for_timer_dismiss = shared_settings.clone();
    let model_for_timer_dismiss = timer_model.clone();
    let attention_for_timer_dismiss = attention_window.as_weak();
    let owner_for_timer_dismiss = owner.as_weak();
    attention_window.on_request_dismiss(move || {
        let Some(item) = queue_for_timer_dismiss.borrow().front().cloned() else {
            return;
        };
        let mut settings = settings_for_timer_dismiss.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::DismissTimer { id: item.timer_id },
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            sound_for_timer_dismiss.stop();
            model_for_timer_dismiss.set_vec(planner_timer_rows(&settings));
            queue_for_timer_dismiss.borrow_mut().pop_front();
            display_timer_attention(
                &attention_for_timer_dismiss,
                &owner_for_timer_dismiss,
                &queue_for_timer_dismiss,
            );
        }
    });

    let queue_for_timer_restart = attention_queue.clone();
    let sound_for_timer_restart = alert_sound.clone();
    let settings_for_timer_restart = shared_settings.clone();
    let model_for_timer_restart = timer_model.clone();
    let attention_for_timer_restart = attention_window.as_weak();
    let owner_for_timer_restart = owner.as_weak();
    attention_window.on_request_restart(move || {
        let Some(item) = queue_for_timer_restart.borrow().front().cloned() else {
            return;
        };
        let mut settings = settings_for_timer_restart.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::RestartTimer {
                id: item.timer_id,
                started_at_utc: Utc::now().to_rfc3339(),
            },
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            sound_for_timer_restart.stop();
            model_for_timer_restart.set_vec(planner_timer_rows(&settings));
            queue_for_timer_restart.borrow_mut().pop_front();
            display_timer_attention(
                &attention_for_timer_restart,
                &owner_for_timer_restart,
                &queue_for_timer_restart,
            );
        }
    });
    display_timer_attention(
        &attention_window.as_weak(),
        &owner.as_weak(),
        &attention_queue,
    );
    schedule_timer_attention_pulse(Rc::new(Timer::default()), attention_window.as_weak());

    let planner_for_edit_timer_part = planner_window.as_weak();
    let settings_for_edit_timer = shared_settings.clone();
    planner_window.on_request_edit_timer_part(move |part, text| {
        let Some(masked) = mask_timer_part(&text, part) else {
            return;
        };
        let Some(planner) = planner_for_edit_timer_part.upgrade() else {
            return;
        };
        match part {
            0 => planner.set_timer_days_draft(masked.clone().into()),
            1 => planner.set_timer_hours_draft(masked.clone().into()),
            2 => planner.set_timer_minutes_draft(masked.clone().into()),
            3 => planner.set_timer_seconds_draft(masked.clone().into()),
            _ => return,
        }
        let Some(value) = parse_timer_part(&masked, part) else {
            return;
        };
        let mut settings = settings_for_edit_timer.borrow_mut();
        store_timer_part(&mut settings, part, value);
        let _ = oziclock_storage::save(&settings);
    });

    let planner_for_adjust_timer = planner_window.as_weak();
    let settings_for_adjust_timer = shared_settings.clone();
    planner_window.on_request_adjust_timer_part(move |part, direction| {
        let Some(planner) = planner_for_adjust_timer.upgrade() else {
            return;
        };
        let text = match part {
            0 => planner.get_timer_days_draft(),
            1 => planner.get_timer_hours_draft(),
            2 => planner.get_timer_minutes_draft(),
            3 => planner.get_timer_seconds_draft(),
            _ => return,
        };
        let Some(value) = adjust_timer_part(&text, part, direction) else {
            return;
        };
        let mut settings = settings_for_adjust_timer.borrow_mut();
        store_timer_part(&mut settings, part, value);
        match part {
            0 => planner.set_timer_days_draft(value.to_string().into()),
            1 => planner.set_timer_hours_draft(value.to_string().into()),
            2 => planner.set_timer_minutes_draft(value.to_string().into()),
            _ => planner.set_timer_seconds_draft(value.to_string().into()),
        }
        let _ = oziclock_storage::save(&settings);
    });

    let planner_for_add_timer = planner_window.as_weak();
    let settings_for_add_timer = shared_settings.clone();
    let timer_model_for_add_timer = timer_model.clone();
    let queue_for_add_timer = attention_queue.clone();
    let attention_for_add_timer = attention_window.as_weak();
    let owner_for_add_timer = owner.as_weak();
    planner_window.on_request_add_timer(move |title, days, hours, minutes, seconds, repeat| {
        let Some(planner) = planner_for_add_timer.upgrade() else {
            return;
        };
        planner.set_timer_error("".into());
        let title = title.trim();
        if title.is_empty() {
            planner.set_timer_error("Enter a timer name.".into());
            return;
        }
        let Some((days, hours, minutes, seconds)) =
            parse_timer_duration(&days, &hours, &minutes, &seconds)
        else {
            planner.set_timer_error("Enter a valid duration.".into());
            return;
        };
        let duration_seconds = u64::from(days) * 86_400
            + u64::from(hours) * 3_600
            + u64::from(minutes) * 60
            + u64::from(seconds);
        if duration_seconds == 0 {
            planner.set_timer_error("Duration must be longer than zero.".into());
            return;
        }
        let repeat_count = if !repeat {
            Some(0)
        } else if planner.get_timer_repeat_unlimited() {
            None
        } else {
            let Ok(count) = planner.get_timer_repeat_count().trim().parse::<u32>() else {
                planner.set_timer_error("Enter a positive repeat count or choose ∞.".into());
                return;
            };
            if count == 0 {
                planner.set_timer_error("Repeat count must be greater than zero.".into());
                return;
            }
            Some(count)
        };
        let mut settings = settings_for_add_timer.borrow_mut();
        let mut updated = settings.clone();
        let editing = planner.get_editing_timer();
        let editing_id = PlannerId::new(editing.to_string());
        let changed = if editing.is_empty() {
            let id = PlannerId::new(format!(
                "timer-{}",
                Utc::now().timestamp_nanos_opt().unwrap_or_default()
            ))
            .expect("generated timer id is valid");
            updated.planner.timers.push(PlannerTimer {
                id,
                title: title.to_owned(),
                duration_seconds,
                remaining_seconds: duration_seconds,
                state: TimerState::Idle,
                repeat,
                repeat_count,
                repeats_remaining: repeat_count,
                started_at_utc: None,
                attention_pending: false,
                attention_triggered_at_utc: None,
            });
            true
        } else {
            let Some(id) = editing_id.clone() else {
                return;
            };
            execute_planner_command(
                &mut updated.planner,
                PlannerCommand::UpdateTimer {
                    id,
                    title: title.to_owned(),
                    duration_seconds,
                    repeat,
                    repeat_count,
                },
            )
        };
        if !changed {
            return;
        }
        updated.timer_draft_days = days;
        updated.timer_draft_hours = hours;
        updated.timer_draft_minutes = minutes;
        updated.timer_draft_seconds = seconds;
        if let Err(error) = oziclock_storage::save(&updated) {
            planner.set_timer_error(format!("Save failed: {error}").into());
            return;
        }
        *settings = updated;
        timer_model_for_add_timer.set_vec(planner_timer_rows(&settings));
        if let Some(id) = editing_id {
            queue_for_add_timer
                .borrow_mut()
                .retain(|item| item.timer_id != id);
            display_timer_attention(
                &attention_for_add_timer,
                &owner_for_add_timer,
                &queue_for_add_timer,
            );
        }
        planner.set_timer_days_draft(days.to_string().into());
        planner.set_timer_hours_draft(hours.to_string().into());
        planner.set_timer_minutes_draft(minutes.to_string().into());
        planner.set_timer_seconds_draft(seconds.to_string().into());
        planner.set_editing_timer("".into());
        planner.set_selected_timer("".into());
        planner.set_timer_title_draft("Timer".into());
        planner.set_timer_repeat(false);
        planner.set_timer_repeat_count("1".into());
        planner.set_timer_repeat_unlimited(false);
        planner.set_editor_modal(-1);
    });

    let settings_for_edit_timer_item = shared_settings.clone();
    let planner_for_edit_timer_item = planner_window.as_weak();
    planner_window.on_request_edit_timer(move |timer_id| {
        let settings = settings_for_edit_timer_item.borrow();
        let Some(timer) = settings
            .planner
            .timers
            .iter()
            .find(|timer| timer.id.to_string() == timer_id.as_str())
        else {
            return;
        };
        let Some(planner) = planner_for_edit_timer_item.upgrade() else {
            return;
        };
        let (days, hours, minutes, seconds) = duration_parts(timer.duration_seconds);
        planner.set_editing_timer(timer_id);
        planner.set_timer_error("".into());
        planner.set_timer_title_draft(timer.title.clone().into());
        planner.set_timer_repeat(timer.repeat);
        planner.set_timer_repeat_count(timer.repeat_count.unwrap_or(1).to_string().into());
        planner.set_timer_repeat_unlimited(timer.repeat && timer.repeat_count.is_none());
        planner.set_timer_days_draft(days.to_string().into());
        planner.set_timer_hours_draft(hours.to_string().into());
        planner.set_timer_minutes_draft(minutes.to_string().into());
        planner.set_timer_seconds_draft(seconds.to_string().into());
    });

    let settings_for_reset_timer = shared_settings.clone();
    let model_for_reset_timer = timer_model.clone();
    let queue_for_reset_timer = attention_queue.clone();
    let attention_for_reset_timer = attention_window.as_weak();
    let owner_for_reset_timer = owner.as_weak();
    planner_window.on_request_reset_timer(move |timer_id| {
        let Some(id) = PlannerId::new(timer_id.to_string()) else {
            return;
        };
        let mut settings = settings_for_reset_timer.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::ResetTimer { id: id.clone() },
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            model_for_reset_timer.set_vec(planner_timer_rows(&settings));
            queue_for_reset_timer
                .borrow_mut()
                .retain(|item| item.timer_id != id);
            display_timer_attention(
                &attention_for_reset_timer,
                &owner_for_reset_timer,
                &queue_for_reset_timer,
            );
        }
    });

    let settings_for_delete_timer = shared_settings.clone();
    let model_for_delete_timer = timer_model.clone();
    let planner_for_delete_timer = planner_window.as_weak();
    let queue_for_delete_timer = attention_queue.clone();
    let attention_for_delete_timer = attention_window.as_weak();
    let owner_for_delete_timer = owner.as_weak();
    planner_window.on_request_delete_timer(move |timer_id| {
        let Some(id) = PlannerId::new(timer_id.to_string()) else {
            return;
        };
        let mut settings = settings_for_delete_timer.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::DeleteTimer { id: id.clone() },
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            model_for_delete_timer.set_vec(planner_timer_rows(&settings));
            queue_for_delete_timer
                .borrow_mut()
                .retain(|item| item.timer_id != id);
            display_timer_attention(
                &attention_for_delete_timer,
                &owner_for_delete_timer,
                &queue_for_delete_timer,
            );
            if let Some(planner) = planner_for_delete_timer.upgrade() {
                if planner.get_selected_timer() == timer_id {
                    planner.set_selected_timer("".into());
                }
                if planner.get_editing_timer() == timer_id {
                    planner.set_editing_timer("".into());
                }
            }
        }
    });

    let settings_for_toggle_timer = shared_settings.clone();
    let timer_model_for_toggle_timer = timer_model.clone();
    let queue_for_toggle_timer = attention_queue.clone();
    let attention_for_toggle_timer = attention_window.as_weak();
    let owner_for_toggle_timer = owner.as_weak();
    planner_window.on_request_toggle_timer(move |timer_id| {
        let Some(id) = PlannerId::new(timer_id.to_string()) else {
            return;
        };
        let mut settings = settings_for_toggle_timer.borrow_mut();
        let now = Utc::now();
        let command = settings
            .planner
            .timers
            .iter()
            .find(|timer| timer.id == id)
            .map(|timer| {
                if timer.state == TimerState::Running {
                    PlannerCommand::PauseTimer {
                        id: id.clone(),
                        remaining_seconds: oziclock_app::timer_time::remaining_seconds(timer, now),
                    }
                } else {
                    PlannerCommand::StartTimer {
                        id: id.clone(),
                        started_at_utc: now.to_rfc3339(),
                    }
                }
            });
        let mut updated = settings.clone();
        if let Some(command) = command
            && execute_planner_command(&mut updated.planner, command)
            && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            timer_model_for_toggle_timer.set_vec(planner_timer_rows(&settings));
            queue_for_toggle_timer
                .borrow_mut()
                .retain(|item| item.timer_id != id);
            display_timer_attention(
                &attention_for_toggle_timer,
                &owner_for_toggle_timer,
                &queue_for_toggle_timer,
            );
        }
    });

    schedule_planner_timer_refresh(
        alert_sound,
        Rc::new(Timer::default()),
        timer_model,
        shared_settings,
        attention_window.as_weak(),
        owner.as_weak(),
        attention_queue,
    );
}

#[derive(Clone)]
struct TimerAttentionItem {
    timer_id: PlannerId,
    title: String,
    triggered_at: String,
}

fn pending_timer_attention_items(settings: &AppSettings) -> VecDeque<TimerAttentionItem> {
    oziclock_app::timer_time::pending_attention(&settings.planner)
        .into_iter()
        .filter_map(|id| {
            settings
                .planner
                .timers
                .iter()
                .find(|timer| timer.id == id)
                .map(|timer| TimerAttentionItem {
                    timer_id: id,
                    title: timer_display_title(timer),
                    triggered_at: timer_triggered_label(timer, settings),
                })
        })
        .collect()
}

fn display_timer_attention(
    window: &slint::Weak<TimerAttentionWindow>,
    owner: &slint::Weak<AppWindow>,
    queue: &Rc<RefCell<VecDeque<TimerAttentionItem>>>,
) {
    let Some(window) = window.upgrade() else {
        return;
    };
    let Some(item) = queue.borrow().front().cloned() else {
        let _ = window.hide();
        return;
    };
    window.set_timer_title(item.title.into());
    window.set_triggered_at(item.triggered_at.into());
    if window.show().is_ok() {
        hide_auxiliary_window_from_taskbar(window.window());
        position_calendar_window(window.window(), owner);
    }
}

fn schedule_timer_attention_pulse(timer: Rc<Timer>, window: slint::Weak<TimerAttentionWindow>) {
    let next_timer = timer.clone();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(450),
        move || {
            if let Some(window) = window.upgrade() {
                window.set_pulse(!window.get_pulse());
            }
            schedule_timer_attention_pulse(next_timer.clone(), window.clone());
        },
    );
}

fn timer_triggered_label(timer: &PlannerTimer, settings: &AppSettings) -> String {
    let Some(triggered) = timer
        .attention_triggered_at_utc
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
    else {
        return String::new();
    };
    let zone = settings
        .clocks_settings
        .iter()
        .find(|clock| clock.is_main)
        .or_else(|| settings.clocks_settings.first())
        .and_then(|clock| clock.time_zone.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    triggered
        .with_timezone(&zone)
        .format("%d %b · %H:%M")
        .to_string()
}

fn schedule_planner_timer_refresh(
    alert_sound: AlertSound,
    timer: Rc<Timer>,
    model: Rc<VecModel<PlannerTimerData>>,
    settings: Rc<RefCell<AppSettings>>,
    attention: slint::Weak<TimerAttentionWindow>,
    owner: slint::Weak<AppWindow>,
    queue: Rc<RefCell<VecDeque<TimerAttentionItem>>>,
) {
    let next_timer = timer.clone();
    let next_model = model.clone();
    let next_settings = settings.clone();
    timer.start(TimerMode::SingleShot, Duration::from_secs(1), move || {
        let mut settings = next_settings.borrow_mut();
        let mut updated = settings.clone();
        let timers_before = updated.planner.timers.clone();
        let finished_ids = oziclock_app::timer_time::reconcile(&mut updated.planner, Utc::now());
        let timers_changed = timers_before != updated.planner.timers;
        if timers_changed && oziclock_storage::save(&updated).is_ok() {
            *settings = updated;
            let was_empty = queue.borrow().is_empty();
            for id in finished_ids {
                if let Some(timer) = settings.planner.timers.iter().find(|timer| timer.id == id) {
                    let title = timer_display_title(timer);
                    let triggered_at = timer_triggered_label(timer, &settings);
                    delivery_feedback::deliver(
                        &title,
                        &triggered_at,
                        settings.alert_sound_duration_seconds,
                        &alert_sound,
                    );
                    queue.borrow_mut().push_back(TimerAttentionItem {
                        timer_id: id,
                        title,
                        triggered_at,
                    });
                }
            }
            if was_empty && !queue.borrow().is_empty() {
                display_timer_attention(&attention, &owner, &queue);
            }
        }
        if settings
            .planner
            .timers
            .iter()
            .any(|timer| timer.state == TimerState::Running || timer.state == TimerState::Finished)
        {
            next_model.set_vec(planner_timer_rows(&settings));
        }
        drop(settings);
        schedule_planner_timer_refresh(
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
