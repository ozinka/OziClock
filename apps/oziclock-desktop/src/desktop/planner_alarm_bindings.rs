use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use oziclock_app::planner::{PlannerCommand, execute_planner_command};
use oziclock_storage::{Alarm, AlarmOccurrenceStatus, AlarmSchedule, AppSettings, PlannerId};
use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel};

use super::{
    AlarmAttentionWindow, AppWindow, PlannerAlarmData, PlannerWindow,
    alert_sound::AlertSound,
    delivery_feedback, hide_auxiliary_window_from_taskbar,
    planner_inputs::{adjust_timer_part, mask_timer_part, normalize_alarm_time},
    planner_models::planner_alarm_rows,
    position_calendar_window,
};

pub(super) fn wire_alarm_bindings(
    alert_sound: AlertSound,
    planner_window: &PlannerWindow,
    attention_window: &AlarmAttentionWindow,
    owner: &AppWindow,
    shared_settings: Rc<RefCell<AppSettings>>,
) {
    let alarm_model = Rc::new(VecModel::from(planner_alarm_rows(
        &shared_settings.borrow(),
    )));
    planner_window.set_alarms(ModelRc::from(alarm_model.clone()));

    let edit_settings = shared_settings.clone();
    let edit_window = planner_window.as_weak();
    planner_window.on_request_edit_alarm(move |id| {
        let settings = edit_settings.borrow();
        let Some(alarm) = settings
            .planner
            .alarms
            .iter()
            .find(|alarm| alarm.id.to_string() == id.as_str())
        else {
            return;
        };
        let Some(ui) = edit_window.upgrade() else {
            return;
        };
        let (time, days) = match &alarm.schedule {
            AlarmSchedule::Once { local_time, .. } => (local_time, None),
            AlarmSchedule::Weekly {
                local_time,
                weekdays,
            } => (local_time, Some(weekdays)),
        };
        let Some(time) = normalize_alarm_time(time) else {
            return;
        };
        ui.set_editing_alarm(id);
        ui.set_alarm_error("".into());
        ui.set_alarm_title_draft(alarm.title.clone().into());
        ui.set_alarm_hours_draft(time[..2].into());
        ui.set_alarm_minutes_draft(time[3..].into());
        ui.set_alarm_weekly(days.is_some());
        let selected = |day| days.is_some_and(|days| days.contains(&day));
        ui.set_alarm_mon(selected(0));
        ui.set_alarm_tue(selected(1));
        ui.set_alarm_wed(selected(2));
        ui.set_alarm_thu(selected(3));
        ui.set_alarm_fri(selected(4));
        ui.set_alarm_sat(selected(5));
        ui.set_alarm_sun(selected(6));
    });

    let delete_settings = shared_settings.clone();
    let delete_model = alarm_model.clone();
    let delete_window = planner_window.as_weak();
    planner_window.on_request_delete_alarm(move |id| {
        let Some(key) = PlannerId::new(id.to_string()) else {
            return;
        };
        let mut settings = delete_settings.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::DeleteAlarm { id: key },
        ) {
            if let Err(error) = oziclock_storage::save(&updated) {
                if let Some(ui) = delete_window.upgrade() {
                    ui.set_alarm_error(format!("Save failed: {error}").into());
                }
                return;
            }
            *settings = updated;
            delete_model.set_vec(planner_alarm_rows(&settings));
            if let Some(ui) = delete_window.upgrade() {
                if ui.get_selected_alarm() == id {
                    ui.set_selected_alarm("".into());
                }
                if ui.get_editing_alarm() == id {
                    ui.set_editing_alarm("".into());
                    ui.set_alarm_title_draft("Alarm".into());
                }
            }
        }
    });

    let planner_for_adjust_alarm = planner_window.as_weak();
    planner_window.on_request_adjust_alarm_part(move |part, direction| {
        let Some(planner) = planner_for_adjust_alarm.upgrade() else {
            return;
        };
        let text = match part {
            1 => planner.get_alarm_hours_draft(),
            2 => planner.get_alarm_minutes_draft(),
            _ => return,
        };
        let Some(value) = adjust_timer_part(&text, part, direction) else {
            return;
        };
        let value = format!("{value:02}").into();
        if part == 1 {
            planner.set_alarm_hours_draft(value);
        } else {
            planner.set_alarm_minutes_draft(value);
        }
    });

    let planner_for_edit_alarm = planner_window.as_weak();
    planner_window.on_request_edit_alarm_part(move |part, text| {
        let Some(planner) = planner_for_edit_alarm.upgrade() else {
            return;
        };
        let Some(value) = mask_timer_part(&text, part) else {
            return;
        };
        if part == 1 {
            planner.set_alarm_hours_draft(value.into());
        } else if part == 2 {
            planner.set_alarm_minutes_draft(value.into());
        }
    });

    let planner_for_add_alarm = planner_window.as_weak();
    let settings_for_add_alarm = shared_settings.clone();
    let alarm_model_for_add_alarm = alarm_model.clone();
    planner_window.on_request_add_alarm(
        move |title, time, weekly, mon, tue, wed, thu, fri, sat, sun| {
            let Some(ui) = planner_for_add_alarm.upgrade() else {
                return;
            };
            ui.set_alarm_error("".into());
            let title = title.trim();
            let Some(time) = normalize_alarm_time(&time) else {
                ui.set_alarm_error("Enter a valid time.".into());
                return;
            };
            if title.is_empty() {
                ui.set_alarm_error("Enter an alarm name.".into());
                return;
            }
            let weekdays = [mon, tue, wed, thu, fri, sat, sun]
                .into_iter()
                .enumerate()
                .filter_map(|(index, selected)| selected.then_some(index as u8))
                .collect::<Vec<_>>();
            if weekly && weekdays.is_empty() {
                ui.set_alarm_error("Select at least one weekday.".into());
                return;
            }
            let mut settings = settings_for_add_alarm.borrow_mut();
            let id = PlannerId::new(format!(
                "alarm-{}",
                Utc::now().timestamp_nanos_opt().unwrap_or_default()
            ))
            .expect("generated alarm id is valid");
            let editing = ui.get_editing_alarm();
            let existing = settings
                .planner
                .alarms
                .iter()
                .find(|alarm| alarm.id.to_string() == editing.as_str())
                .cloned();
            if !editing.is_empty() && existing.is_none() {
                return;
            }
            let time_zone = existing
                .as_ref()
                .map(|alarm| alarm.time_zone.clone())
                .unwrap_or_else(|| {
                    settings
                        .clocks_settings
                        .iter()
                        .find(|clock| clock.is_main)
                        .or_else(|| settings.clocks_settings.first())
                        .map(|clock| clock.time_zone.clone())
                        .unwrap_or_else(|| "UTC".to_owned())
                });
            let schedule = if weekly {
                AlarmSchedule::Weekly {
                    local_time: time.to_owned(),
                    weekdays,
                }
            } else {
                let Some((date, _)) =
                    oziclock_app::alarm_time::next_once(&time, &time_zone, Utc::now())
                else {
                    return;
                };
                AlarmSchedule::Once {
                    local_date: date.to_string(),
                    local_time: time.to_owned(),
                }
            };
            let alarm = Alarm {
                id: existing
                    .as_ref()
                    .map(|alarm| alarm.id.clone())
                    .unwrap_or(id),
                title: title.to_owned(),
                schedule,
                time_zone,
                enabled: existing.as_ref().is_none_or(|alarm| {
                    oziclock_app::alarm_time::enabled_after_edit(
                        alarm,
                        &settings.planner.alarm_receipts,
                    )
                }),
            };
            let mut updated = settings.clone();
            if let Some(item) = updated
                .planner
                .alarms
                .iter_mut()
                .find(|item| item.id == alarm.id)
            {
                *item = alarm;
            } else {
                updated.planner.alarms.push(alarm);
            }
            if let Err(error) = oziclock_storage::save(&updated) {
                ui.set_alarm_error(format!("Save failed: {error}").into());
                return;
            }
            *settings = updated;
            ui.set_editing_alarm("".into());
            ui.set_selected_alarm("".into());
            ui.set_editor_modal(-1);
            alarm_model_for_add_alarm.set_vec(planner_alarm_rows(&settings));
            if let Some(planner) = planner_for_add_alarm.upgrade() {
                planner.set_alarm_title_draft("Alarm".into());
                planner.set_alarm_hours_draft(time[..2].into());
                planner.set_alarm_minutes_draft(time[3..].into());
            }
        },
    );

    let settings_for_toggle_alarm = shared_settings.clone();
    let alarm_model_for_toggle_alarm = alarm_model.clone();
    planner_window.on_request_toggle_alarm(move |alarm_id, enabled| {
        let Some(id) = PlannerId::new(alarm_id.to_string()) else {
            return;
        };
        let mut settings = settings_for_toggle_alarm.borrow_mut();
        let mut updated = settings.clone();
        if updated
            .planner
            .alarms
            .iter_mut()
            .find(|alarm| alarm.id == id)
            .is_some_and(|alarm| oziclock_app::alarm_time::set_enabled(alarm, enabled, Utc::now()))
            && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            alarm_model_for_toggle_alarm.set_vec(planner_alarm_rows(&settings));
        }
    });

    let attention_queue = Rc::new(RefCell::new(pending_alarm_attention_items(
        &shared_settings.borrow(),
    )));
    let queue_for_dismiss = attention_queue.clone();
    let sound_for_alarm_dismiss = alert_sound.clone();
    let settings_for_dismiss = shared_settings.clone();
    let attention_for_dismiss = attention_window.as_weak();
    let owner_for_dismiss = owner.as_weak();
    attention_window.on_request_dismiss(move || {
        let Some(item) = queue_for_dismiss.borrow().front().cloned() else {
            return;
        };
        let mut settings = settings_for_dismiss.borrow_mut();
        let mut updated = settings.clone();
        if oziclock_app::alarm_time::acknowledge_attention(
            &mut updated.planner,
            &item.alarm_id,
            &item.occurrence_utc,
            Utc::now(),
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            sound_for_alarm_dismiss.stop();
            queue_for_dismiss.borrow_mut().pop_front();
            display_alarm_attention(
                &attention_for_dismiss,
                &owner_for_dismiss,
                &queue_for_dismiss,
            );
        }
    });

    let queue_for_snooze = attention_queue.clone();
    let sound_for_snooze = alert_sound.clone();
    let settings_for_snooze = shared_settings.clone();
    let attention_for_snooze = attention_window.as_weak();
    let owner_for_snooze = owner.as_weak();
    attention_window.on_request_snooze(move |minutes| {
        let Some(item) = queue_for_snooze.borrow().front().cloned() else {
            return;
        };
        let mut settings = settings_for_snooze.borrow_mut();
        let mut updated = settings.clone();
        let now = Utc::now();
        let minutes = minutes.round() as u16;
        if oziclock_app::alarm_time::snooze(&mut updated.planner, &item.alarm_id, now, minutes)
            && oziclock_app::alarm_time::acknowledge_attention(
                &mut updated.planner,
                &item.alarm_id,
                &item.occurrence_utc,
                now,
            )
            && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            sound_for_snooze.stop();
            queue_for_snooze.borrow_mut().pop_front();
            display_alarm_attention(&attention_for_snooze, &owner_for_snooze, &queue_for_snooze);
        }
    });

    schedule_alarm_attention_pulse(Rc::new(Timer::default()), attention_window.as_weak());
    display_alarm_attention(
        &attention_window.as_weak(),
        &owner.as_weak(),
        &attention_queue,
    );
    schedule_alarm_refresh(
        alert_sound,
        Rc::new(Timer::default()),
        alarm_model,
        shared_settings,
        attention_window.as_weak(),
        owner.as_weak(),
        attention_queue,
    );
}

#[derive(Clone)]
struct AlarmAttentionItem {
    alarm_id: PlannerId,
    occurrence_utc: String,
    title: String,
    time: String,
}

fn pending_alarm_attention_items(settings: &AppSettings) -> VecDeque<AlarmAttentionItem> {
    oziclock_app::alarm_time::pending_attention(&settings.planner)
        .into_iter()
        .filter_map(|receipt| {
            settings
                .planner
                .alarms
                .iter()
                .find(|alarm| alarm.id == receipt.alarm_id)
                .map(|alarm| AlarmAttentionItem {
                    alarm_id: receipt.alarm_id,
                    occurrence_utc: receipt.occurrence_utc,
                    title: alarm.title.clone(),
                    time: alarm_time_label(alarm),
                })
        })
        .collect()
}

fn display_alarm_attention(
    window: &slint::Weak<AlarmAttentionWindow>,
    owner: &slint::Weak<AppWindow>,
    queue: &Rc<RefCell<VecDeque<AlarmAttentionItem>>>,
) {
    let Some(window) = window.upgrade() else {
        return;
    };
    let Some(item) = queue.borrow().front().cloned() else {
        let _ = window.hide();
        return;
    };
    window.set_alarm_title(item.title.into());
    window.set_alarm_time(item.time.into());
    if window.show().is_ok() {
        hide_auxiliary_window_from_taskbar(window.window());
        position_calendar_window(window.window(), owner);
    }
}

fn schedule_alarm_attention_pulse(timer: Rc<Timer>, window: slint::Weak<AlarmAttentionWindow>) {
    let next_timer = timer.clone();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(450),
        move || {
            if let Some(window) = window.upgrade() {
                window.set_pulse(!window.get_pulse());
            }
            schedule_alarm_attention_pulse(next_timer.clone(), window.clone());
        },
    );
}

fn schedule_alarm_refresh(
    alert_sound: AlertSound,
    timer: Rc<Timer>,
    model: Rc<VecModel<PlannerAlarmData>>,
    settings: Rc<RefCell<AppSettings>>,
    attention: slint::Weak<AlarmAttentionWindow>,
    owner: slint::Weak<AppWindow>,
    queue: Rc<RefCell<VecDeque<AlarmAttentionItem>>>,
) {
    let next_timer = timer.clone();
    let next_settings = settings.clone();
    timer.start(TimerMode::SingleShot, Duration::from_secs(1), move || {
        let now = Utc::now();
        let mut settings_guard = next_settings.borrow_mut();
        let after = settings_guard
            .planner
            .alarm_checked_at_utc
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or(now - chrono::Duration::seconds(1));
        let titles = settings_guard
            .planner
            .alarms
            .iter()
            .map(|alarm| {
                (
                    alarm.id.clone(),
                    (alarm.title.clone(), alarm_time_label(alarm)),
                )
            })
            .collect::<HashMap<_, _>>();
        let mut updated = settings_guard.clone();
        let mut receipts = oziclock_app::alarm_time::reconcile(
            &mut updated.planner,
            after,
            now,
            chrono::Duration::minutes(5),
        );
        receipts.extend(oziclock_app::alarm_time::reconcile_snoozes(
            &mut updated.planner,
            now,
            chrono::Duration::minutes(5),
        ));
        oziclock_storage::prune_alarm_receipts(&mut updated, now);
        if oziclock_storage::save(&updated).is_ok() {
            *settings_guard = updated;
            model.set_vec(planner_alarm_rows(&settings_guard));
            let was_empty = queue.borrow().is_empty();
            for receipt in receipts
                .iter()
                .filter(|receipt| receipt.status == AlarmOccurrenceStatus::Delivered)
            {
                if let Some((title, time)) = titles.get(&receipt.alarm_id) {
                    delivery_feedback::deliver(
                        title,
                        time,
                        settings_guard.alert_sound_duration_seconds,
                        &alert_sound,
                    );
                    queue.borrow_mut().push_back(AlarmAttentionItem {
                        alarm_id: receipt.alarm_id.clone(),
                        occurrence_utc: receipt.occurrence_utc.clone(),
                        title: title.clone(),
                        time: time.clone(),
                    });
                }
            }
            if was_empty && !queue.borrow().is_empty() {
                display_alarm_attention(&attention, &owner, &queue);
            }
        }
        drop(settings_guard);
        schedule_alarm_refresh(
            alert_sound.clone(),
            next_timer.clone(),
            model.clone(),
            next_settings.clone(),
            attention.clone(),
            owner.clone(),
            queue.clone(),
        );
    });
}

fn alarm_time_label(alarm: &Alarm) -> String {
    match &alarm.schedule {
        AlarmSchedule::Once { local_time, .. } | AlarmSchedule::Weekly { local_time, .. } => {
            format!("{local_time} · {}", alarm.time_zone)
        }
    }
}
