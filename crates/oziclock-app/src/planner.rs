use oziclock_domain::{Planner, PlannerId, TimerState};

/// Typed Planner intents issued by presentation adapters.
pub enum PlannerCommand {
    DeleteAlarm {
        id: PlannerId,
    },
    CompleteTask {
        id: PlannerId,
    },
    ArchiveTask {
        id: PlannerId,
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
    StartStopwatch,
    PauseStopwatch {
        elapsed_milliseconds: u64,
    },
    ResetStopwatch,
    RecordStopwatchLap {
        elapsed_seconds: u64,
    },
}

/// Applies a Planner use case while preserving the domain state machine.
pub fn execute_planner_command(planner: &mut Planner, command: PlannerCommand) -> bool {
    match command {
        PlannerCommand::DeleteAlarm { id } => {
            let count = planner.alarms.len();
            planner.alarms.retain(|alarm| alarm.id != id);
            let changed = count != planner.alarms.len();
            if changed {
                planner
                    .alarm_receipts
                    .retain(|receipt| receipt.alarm_id != id);
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
        PlannerCommand::RecordStopwatchLap { elapsed_seconds } => planner
            .stopwatch
            .as_mut()
            .is_some_and(|stopwatch| stopwatch.record_lap(elapsed_seconds)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oziclock_domain::{Task, TaskStatus};

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
    }
}
