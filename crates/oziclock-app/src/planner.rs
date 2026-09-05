use oziclock_domain::{Planner, PlannerId, ReminderSchedule, TimerState};

/// Typed Planner intents issued by presentation adapters.
pub enum PlannerCommand {
    SetReminderEnabled {
        id: PlannerId,
        enabled: bool,
    },
    DeleteReminder {
        id: PlannerId,
    },
    DismissReminder {
        id: PlannerId,
    },
    UpdateReminder {
        id: PlannerId,
        title: String,
        schedule: ReminderSchedule,
    },
    DeleteAlarm {
        id: PlannerId,
    },
    CompleteTask {
        id: PlannerId,
    },
    ArchiveTask {
        id: PlannerId,
    },
    ReopenTask {
        id: PlannerId,
    },
    DeleteTask {
        id: PlannerId,
    },
    RenameTask {
        id: PlannerId,
        title: String,
    },
    SetAlarmEnabled {
        id: PlannerId,
        enabled: bool,
    },
    StartTimer {
        id: PlannerId,
        started_at_utc: String,
    },
    PauseTimer {
        id: PlannerId,
        remaining_seconds: u64,
    },
    FinishTimer {
        id: PlannerId,
    },
    DismissTimer {
        id: PlannerId,
    },
    RestartTimer {
        id: PlannerId,
        started_at_utc: String,
    },
    ResetTimer {
        id: PlannerId,
    },
    DeleteTimer {
        id: PlannerId,
    },
    UpdateTimer {
        id: PlannerId,
        title: String,
        duration_seconds: u64,
        repeat: bool,
    },
    StartStopwatch,
    PauseStopwatch {
        elapsed_milliseconds: u64,
    },
    ResetStopwatch,
    RecordStopwatchLap {
        elapsed_milliseconds: u64,
    },
    UndoStopwatchLap,
    ClearStopwatchLaps,
}

/// Applies a Planner use case while preserving the domain state machine.
pub fn execute_planner_command(planner: &mut Planner, command: PlannerCommand) -> bool {
    match command {
        PlannerCommand::SetReminderEnabled { id, enabled } => planner
            .reminders
            .iter_mut()
            .find(|reminder| reminder.id == id)
            .is_some_and(|reminder| reminder.set_enabled(enabled)),
        PlannerCommand::DeleteReminder { id } => {
            let count = planner.reminders.len();
            planner.reminders.retain(|reminder| reminder.id != id);
            count != planner.reminders.len()
        }
        PlannerCommand::DismissReminder { id } => planner
            .reminders
            .iter_mut()
            .find(|reminder| reminder.id == id)
            .is_some_and(|reminder| reminder.dismiss()),
        PlannerCommand::UpdateReminder {
            id,
            title,
            schedule,
        } => planner
            .reminders
            .iter_mut()
            .find(|reminder| reminder.id == id)
            .is_some_and(|reminder| reminder.update(title, schedule)),
        PlannerCommand::DeleteAlarm { id } => {
            let count = planner.alarms.len();
            planner.alarms.retain(|alarm| alarm.id != id);
            let changed = count != planner.alarms.len();
            if changed {
                planner
                    .alarm_receipts
                    .retain(|receipt| receipt.alarm_id != id);
                planner.alarm_snoozes.retain(|snooze| snooze.alarm_id != id);
            }
            changed
        }
        PlannerCommand::CompleteTask { id } => planner
            .tasks
            .iter_mut()
            .find(|task| task.id == id)
            .is_some_and(|task| task.complete()),
        PlannerCommand::ArchiveTask { id } => planner
            .tasks
            .iter_mut()
            .find(|task| task.id == id)
            .is_some_and(|task| task.archive()),
        PlannerCommand::ReopenTask { id } => planner
            .tasks
            .iter_mut()
            .find(|task| task.id == id)
            .is_some_and(|task| task.reopen()),
        PlannerCommand::DeleteTask { id } => {
            let count = planner.tasks.len();
            planner.tasks.retain(|task| task.id != id);
            count != planner.tasks.len()
        }
        PlannerCommand::RenameTask { id, title } => planner
            .tasks
            .iter_mut()
            .find(|task| task.id == id)
            .is_some_and(|task| task.rename(title)),
        PlannerCommand::SetAlarmEnabled { id, enabled } => planner
            .alarms
            .iter_mut()
            .find(|alarm| alarm.id == id)
            .is_some_and(|alarm| {
                alarm.set_enabled(enabled);
                true
            }),
        PlannerCommand::StartTimer { id, started_at_utc } => planner
            .timers
            .iter_mut()
            .find(|timer| timer.id == id)
            .is_some_and(|timer| timer.start(started_at_utc)),
        PlannerCommand::PauseTimer {
            id,
            remaining_seconds,
        } => planner
            .timers
            .iter_mut()
            .find(|timer| timer.id == id)
            .is_some_and(|timer| timer.pause(remaining_seconds)),
        PlannerCommand::FinishTimer { id } => {
            let Some(timer) = planner.timers.iter_mut().find(|timer| timer.id == id) else {
                return false;
            };
            if timer.state != TimerState::Running {
                return false;
            }
            timer.finish();
            true
        }
        PlannerCommand::DismissTimer { id } => planner
            .timers
            .iter_mut()
            .find(|timer| timer.id == id)
            .is_some_and(|timer| timer.dismiss()),
        PlannerCommand::RestartTimer { id, started_at_utc } => planner
            .timers
            .iter_mut()
            .find(|timer| timer.id == id)
            .is_some_and(|timer| timer.restart(started_at_utc)),
        PlannerCommand::ResetTimer { id } => planner
            .timers
            .iter_mut()
            .find(|timer| timer.id == id)
            .is_some_and(|timer| timer.reset()),
        PlannerCommand::DeleteTimer { id } => {
            let count = planner.timers.len();
            planner.timers.retain(|timer| timer.id != id);
            count != planner.timers.len()
        }
        PlannerCommand::UpdateTimer {
            id,
            title,
            duration_seconds,
            repeat,
        } => planner
            .timers
            .iter_mut()
            .find(|timer| timer.id == id)
            .is_some_and(|timer| timer.update(title, duration_seconds, repeat)),
        PlannerCommand::StartStopwatch => planner
            .stopwatch
            .as_mut()
            .is_some_and(|stopwatch| stopwatch.start()),
        PlannerCommand::PauseStopwatch {
            elapsed_milliseconds,
        } => planner
            .stopwatch
            .as_mut()
            .is_some_and(|stopwatch| stopwatch.pause(elapsed_milliseconds)),
        PlannerCommand::ResetStopwatch => planner.stopwatch.as_mut().is_some_and(|stopwatch| {
            stopwatch.reset();
            true
        }),
        PlannerCommand::RecordStopwatchLap {
            elapsed_milliseconds,
        } => planner
            .stopwatch
            .as_mut()
            .is_some_and(|stopwatch| stopwatch.record_lap(elapsed_milliseconds)),
        PlannerCommand::UndoStopwatchLap => planner
            .stopwatch
            .as_mut()
            .is_some_and(|stopwatch| stopwatch.undo_lap()),
        PlannerCommand::ClearStopwatchLaps => planner
            .stopwatch
            .as_mut()
            .is_some_and(|stopwatch| stopwatch.clear_laps()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oziclock_domain::{Reminder, Task, TaskStatus, Timer, TimerState};

    fn id(value: &str) -> PlannerId {
        PlannerId::new(value).unwrap()
    }

    #[test]
    fn deletion_removes_only_the_requested_alarm() {
        use oziclock_domain::{Alarm, AlarmSchedule};
        let mut planner = Planner::default();
        for name in ["a", "b"] {
            planner.alarms.push(Alarm {
                id: id(name),
                title: name.into(),
                enabled: true,
                time_zone: "UTC".into(),
                schedule: AlarmSchedule::Once {
                    local_date: "2026-09-04".into(),
                    local_time: "07:00".into(),
                },
            });
        }
        assert!(execute_planner_command(
            &mut planner,
            PlannerCommand::DeleteAlarm { id: id("a") }
        ));
        assert_eq!(planner.alarms.len(), 1);
        assert_eq!(planner.alarms[0].id, id("b"));
        assert!(!execute_planner_command(
            &mut planner,
            PlannerCommand::DeleteAlarm { id: id("a") }
        ));
    }

    #[test]
    fn completing_a_task_uses_the_domain_transition() {
        let mut planner = Planner {
            tasks: vec![Task {
                id: id("task"),
                title: "Send invoice".into(),
                status: TaskStatus::Open,
                due_utc: None,
                scheduled_start_utc: None,
                scheduled_end_utc: None,
                color: None,
                tags: vec![],
                notes: None,
                alerts: vec![],
            }],
            ..Planner::default()
        };

        assert!(execute_planner_command(
            &mut planner,
            PlannerCommand::CompleteTask { id: id("task") }
        ));
        assert_eq!(planner.tasks[0].status, TaskStatus::Completed);
        assert!(execute_planner_command(
            &mut planner,
            PlannerCommand::RenameTask {
                id: id("task"),
                title: "Send final invoice".into(),
            }
        ));
        assert!(execute_planner_command(
            &mut planner,
            PlannerCommand::ReopenTask { id: id("task") }
        ));
        assert_eq!(planner.tasks[0].status, TaskStatus::Open);
        assert!(execute_planner_command(
            &mut planner,
            PlannerCommand::DeleteTask { id: id("task") }
        ));
        assert!(planner.tasks.is_empty());
    }

    fn timer() -> Timer {
        Timer {
            id: id("timer"),
            title: "Focus".into(),
            duration_seconds: 300,
            remaining_seconds: 120,
            state: TimerState::Paused,
            repeat: false,
            started_at_utc: None,
            attention_pending: true,
        }
    }

    #[test]
    fn timer_can_be_updated_reset_restarted_and_deleted() {
        let mut planner = Planner {
            timers: vec![timer()],
            ..Planner::default()
        };
        assert!(execute_planner_command(
            &mut planner,
            PlannerCommand::UpdateTimer {
                id: id("timer"),
                title: "Tea".into(),
                duration_seconds: 600,
                repeat: true,
            }
        ));
        assert_eq!(planner.timers[0].title, "Tea");
        assert_eq!(planner.timers[0].state, TimerState::Idle);
        assert!(planner.timers[0].repeat);

        assert!(execute_planner_command(
            &mut planner,
            PlannerCommand::RestartTimer {
                id: id("timer"),
                started_at_utc: "2026-09-05T10:00:00Z".into(),
            }
        ));
        assert_eq!(planner.timers[0].state, TimerState::Running);
        assert!(execute_planner_command(
            &mut planner,
            PlannerCommand::ResetTimer { id: id("timer") }
        ));
        assert_eq!(planner.timers[0].state, TimerState::Idle);
        assert_eq!(planner.timers[0].remaining_seconds, 600);

        assert!(execute_planner_command(
            &mut planner,
            PlannerCommand::DeleteTimer { id: id("timer") }
        ));
        assert!(planner.timers.is_empty());
    }

    #[test]
    fn delivered_reminder_can_only_be_rearmed_by_editing() {
        let schedule = ReminderSchedule::Absolute {
            at_utc: "2026-09-05T10:00:00Z".into(),
            source_time_zone: "UTC".into(),
        };
        let mut reminder = Reminder {
            id: id("reminder"),
            title: "Stretch".into(),
            schedule: schedule.clone(),
            alerts: vec![],
            enabled: true,
            attention_pending: false,
        };
        assert!(reminder.deliver());
        let mut planner = Planner {
            reminders: vec![reminder],
            ..Planner::default()
        };
        assert!(!execute_planner_command(
            &mut planner,
            PlannerCommand::SetReminderEnabled {
                id: id("reminder"),
                enabled: true,
            }
        ));
        assert!(execute_planner_command(
            &mut planner,
            PlannerCommand::UpdateReminder {
                id: id("reminder"),
                title: "Walk".into(),
                schedule,
            }
        ));
        assert!(planner.reminders[0].enabled);
        assert!(!planner.reminders[0].attention_pending);
    }
}
