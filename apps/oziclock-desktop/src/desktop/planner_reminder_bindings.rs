use std::{cell::RefCell, collections::VecDeque, rc::Rc, time::Duration};

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use oziclock_app::planner::{PlannerCommand, execute_planner_command};
use oziclock_storage::{AppSettings, PlannerId, Reminder, ReminderRecurrence, ReminderSchedule};
use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel};

use super::{
    AppWindow, PlanReminderMarkerData, PlannerReminderData, PlannerWindow, ReminderAttentionWindow,
    alert_sound::AlertSound,
    default_reminder_datetime, delivery_feedback, hide_auxiliary_window_from_taskbar,
    main_time_zone,
    planner_inputs::{adjust_timer_part, mask_timer_part, normalize_alarm_time},
    planner_models::{plan_reminder_rows, planner_reminder_rows},
    position_calendar_window,
};

pub(super) fn wire_reminder_bindings(
    alert_sound: AlertSound,
    planner_window: &PlannerWindow,
    attention_window: &ReminderAttentionWindow,
    owner: &AppWindow,
    shared_settings: Rc<RefCell<AppSettings>>,
    plan_reminder_model: Rc<VecModel<PlanReminderMarkerData>>,
    plan_week_start: Rc<RefCell<NaiveDate>>,
) {
    let reminder_model = Rc::new(VecModel::from(planner_reminder_rows(
        &shared_settings.borrow(),
    )));
    planner_window.set_reminders(ModelRc::from(reminder_model.clone()));
    let attention_queue = Rc::new(RefCell::new(pending_reminder_attention_items(
        &shared_settings.borrow(),
    )));

    let queue_for_reminder_dismiss = attention_queue.clone();
    let sound_for_reminder_dismiss = alert_sound.clone();
    let settings_for_reminder_dismiss = shared_settings.clone();
    let model_for_reminder_dismiss = reminder_model.clone();
    let attention_for_reminder_dismiss = attention_window.as_weak();
    let owner_for_reminder_dismiss = owner.as_weak();
    attention_window.on_request_dismiss(move || {
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
        &attention_window.as_weak(),
        &owner.as_weak(),
        &attention_queue,
    );
    schedule_reminder_attention_pulse(Rc::new(Timer::default()), attention_window.as_weak());

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
    let model_for_add_reminder = reminder_model.clone();
    let plan_model_for_add_reminder = plan_reminder_model.clone();
    let plan_start_for_add_reminder = plan_week_start.clone();
    let queue_for_add_reminder = attention_queue.clone();
    let attention_for_add_reminder = attention_window.as_weak();
    let owner_for_add_reminder = owner.as_weak();
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
    let model_for_toggle_reminder = reminder_model.clone();
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
    let model_for_delete_reminder = reminder_model.clone();
    let plan_model_for_delete_reminder = plan_reminder_model.clone();
    let plan_start_for_delete_reminder = plan_week_start.clone();
    let planner_for_delete_reminder = planner_window.as_weak();
    let queue_for_delete_reminder = attention_queue.clone();
    let attention_for_delete_reminder = attention_window.as_weak();
    let owner_for_delete_reminder = owner.as_weak();
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
        alert_sound,
        Rc::new(Timer::default()),
        reminder_model,
        shared_settings,
        attention_window.as_weak(),
        owner.as_weak(),
        attention_queue,
    );
}

#[derive(Clone)]
struct ReminderAttentionItem {
    reminder_id: PlannerId,
    title: String,
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

fn schedule_reminder_refresh(
    alert_sound: AlertSound,
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
