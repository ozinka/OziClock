use std::{cell::RefCell, collections::VecDeque, rc::Rc, time::Duration};

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use oziclock_app::planner::{PlannerCommand, execute_planner_command};
use oziclock_storage::{AlertRule, AppSettings, Event, EventReceipt, EventTime, PlannerId};
use slint::{ComponentHandle, Timer, TimerMode, VecModel};

use super::{
    AppWindow, AuxiliaryFocusPolicy, EventAttentionWindow, PlanAllDayEventData,
    PlanEventMarkerData, PlannerWindow,
    alert_sound::AlertSound,
    delivery_feedback, main_time_zone,
    planner_inputs::{
        event_alert_label, event_recurrence_label, normalize_alarm_time, parse_alert_with_custom,
        parse_event_recurrence,
    },
    planner_models::plan_event_rows,
    position_calendar_window, show_auxiliary_window,
};

pub(super) struct EventPlanModels {
    events: Rc<VecModel<PlanEventMarkerData>>,
    all_day_events: Rc<VecModel<PlanAllDayEventData>>,
}

impl EventPlanModels {
    pub(super) fn new(
        events: Rc<VecModel<PlanEventMarkerData>>,
        all_day_events: Rc<VecModel<PlanAllDayEventData>>,
    ) -> Self {
        Self {
            events,
            all_day_events,
        }
    }
}

pub(super) fn wire_event_bindings(
    alert_sound: AlertSound,
    planner_window: &PlannerWindow,
    attention_window: &EventAttentionWindow,
    owner: &AppWindow,
    shared_settings: Rc<RefCell<AppSettings>>,
    models: EventPlanModels,
    plan_week_start: Rc<RefCell<NaiveDate>>,
) {
    let event_model = models.events;
    let all_day_event_model = models.all_day_events;
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

    let attention_queue = Rc::new(RefCell::new(pending_event_attention_items(
        &shared_settings.borrow(),
    )));
    let planner_for_save_event = planner_window.as_weak();
    let settings_for_save_event = shared_settings.clone();
    let event_model_for_save_event = event_model.clone();
    let all_day_model_for_save_event = all_day_event_model.clone();
    let start_for_save_event = plan_week_start.clone();
    let queue_for_save_event = attention_queue.clone();
    let attention_for_save_event = attention_window.as_weak();
    let owner_for_save_event = owner.as_weak();
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
            refresh_event_models(
                &settings,
                *start_for_save_event.borrow(),
                &event_model_for_save_event,
                &all_day_model_for_save_event,
            );
            planner.set_selected_plan_event("".into());
            planner.set_editing_event("".into());
            planner.set_editor_modal(-1);
        },
    );

    let planner_for_delete_event = planner_window.as_weak();
    let settings_for_delete_event = shared_settings.clone();
    let event_model_for_delete_event = event_model.clone();
    let all_day_model_for_delete_event = all_day_event_model.clone();
    let start_for_delete_event = plan_week_start.clone();
    let queue_for_delete_event = attention_queue.clone();
    let attention_for_delete_event = attention_window.as_weak();
    let owner_for_delete_event = owner.as_weak();
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
        refresh_event_models(
            &settings,
            *start_for_delete_event.borrow(),
            &event_model_for_delete_event,
            &all_day_model_for_delete_event,
        );
        if let Some(planner) = planner_for_delete_event.upgrade() {
            planner.set_selected_plan_event("".into());
            planner.set_selected_plan_date("".into());
            planner.set_selected_plan_title("".into());
            planner.set_selected_plan_time("".into());
            planner.set_editing_event("".into());
            planner.set_editor_modal(-1);
        }
    });

    let queue_for_event_dismiss = attention_queue.clone();
    let sound_for_event_dismiss = alert_sound.clone();
    let settings_for_event_dismiss = shared_settings.clone();
    let attention_for_event_dismiss = attention_window.as_weak();
    let owner_for_event_dismiss = owner.as_weak();
    attention_window.on_request_dismiss(move || {
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
        &attention_window.as_weak(),
        &owner.as_weak(),
        &attention_queue,
    );
    schedule_event_attention_pulse(Rc::new(Timer::default()), attention_window.as_weak());
    schedule_event_refresh(
        alert_sound,
        Rc::new(Timer::default()),
        shared_settings,
        attention_window.as_weak(),
        owner.as_weak(),
        attention_queue,
    );
}

fn refresh_event_models(
    settings: &AppSettings,
    start: NaiveDate,
    event_model: &VecModel<PlanEventMarkerData>,
    all_day_event_model: &VecModel<PlanAllDayEventData>,
) {
    let (events, all_day_events) = plan_event_rows(settings, start);
    event_model.set_vec(events);
    all_day_event_model.set_vec(all_day_events);
}

#[derive(Clone)]
struct EventAttentionItem {
    event_id: PlannerId,
    occurrence_utc: String,
    title: String,
    time: String,
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
    let _ = show_auxiliary_window(
        window.window(),
        |_| {},
        |window| position_calendar_window(window, owner),
        AuxiliaryFocusPolicy::Preserve,
    );
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

fn schedule_event_refresh(
    alert_sound: AlertSound,
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
