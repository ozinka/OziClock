use std::{cell::RefCell, collections::VecDeque, rc::Rc, time::Duration};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use oziclock_app::planner::{PlannerCommand, execute_planner_command};
use oziclock_storage::{AlertRule, AppSettings, PlannerId, Task, TaskStatus};
use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel};

use super::{
    AppWindow, AuxiliaryFocusPolicy, PlannerTaskData, PlannerWindow, TaskAttentionWindow,
    alert_sound::AlertSound,
    delivery_feedback, main_time_zone,
    planner_inputs::{parse_alert_with_custom, task_alert_label},
    planner_models::{
        completed_task_count, open_task_count, planner_completed_task_rows, planner_task_rows,
    },
    position_calendar_window, show_auxiliary_window,
};

pub(super) fn wire_task_bindings(
    alert_sound: AlertSound,
    planner_window: &PlannerWindow,
    attention_window: &TaskAttentionWindow,
    owner: &AppWindow,
    shared_settings: Rc<RefCell<AppSettings>>,
) {
    let attention_queue = Rc::new(RefCell::new(pending_task_attention_items(
        &shared_settings.borrow(),
    )));
    let queue_for_task_dismiss = attention_queue.clone();
    let sound_for_task_dismiss = alert_sound.clone();
    let settings_for_task_dismiss = shared_settings.clone();
    let attention_for_task_dismiss = attention_window.as_weak();
    let owner_for_task_dismiss = owner.as_weak();
    attention_window.on_request_dismiss(move || {
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
        &attention_window.as_weak(),
        &owner.as_weak(),
        &attention_queue,
    );
    schedule_task_attention_pulse(Rc::new(Timer::default()), attention_window.as_weak());
    schedule_task_refresh(
        alert_sound,
        Rc::new(Timer::default()),
        shared_settings.clone(),
        attention_window.as_weak(),
        owner.as_weak(),
        attention_queue.clone(),
    );

    let task_model = Rc::new(VecModel::from(planner_task_rows(&shared_settings.borrow())));
    let completed_task_model = Rc::new(VecModel::from(planner_completed_task_rows(
        &shared_settings.borrow(),
    )));
    planner_window.set_tasks(ModelRc::from(task_model.clone()));
    planner_window.set_completed_tasks(ModelRc::from(completed_task_model.clone()));

    let planner_for_add_task = planner_window.as_weak();
    let settings_for_add_task = shared_settings.clone();
    let task_model_for_add_task = task_model.clone();
    let completed_model_for_add_task = completed_task_model.clone();
    let queue_for_add_task = attention_queue.clone();
    let attention_for_add_task = attention_window.as_weak();
    let owner_for_add_task = owner.as_weak();
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
    let task_model_for_complete_task = task_model.clone();
    let completed_model_for_complete_task = completed_task_model.clone();
    let queue_for_complete_task = attention_queue.clone();
    let attention_for_complete_task = attention_window.as_weak();
    let owner_for_complete_task = owner.as_weak();
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
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
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
    let open_model_for_reopen_task = task_model.clone();
    let completed_model_for_reopen_task = completed_task_model.clone();
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
    let open_model_for_archive_task = task_model.clone();
    let completed_model_for_archive_task = completed_task_model.clone();
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
    let open_model_for_delete_task = task_model.clone();
    let completed_model_for_delete_task = completed_task_model.clone();
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
    let _ = show_auxiliary_window(
        window.window(),
        |_| {},
        |window| position_calendar_window(window, owner),
        AuxiliaryFocusPolicy::Preserve,
    );
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

fn schedule_task_refresh(
    alert_sound: AlertSound,
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
