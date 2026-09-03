use oziclock_domain::{Planner, PlannerId, TimerState};

/// Typed Planner intents issued by presentation adapters.
pub enum PlannerCommand {
    CompleteTask {
        id: PlannerId,
    },
    ArchiveTask {
        id: PlannerId,
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
        elapsed_seconds: u64,
    },
}

/// Applies a Planner use case while preserving the domain state machine.
pub fn execute_planner_command(planner: &mut Planner, command: PlannerCommand) -> bool {
    match command {
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
        PlannerCommand::PauseStopwatch { elapsed_seconds } => planner
            .stopwatch
            .as_mut()
            .is_some_and(|stopwatch| stopwatch.pause(elapsed_seconds)),
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
