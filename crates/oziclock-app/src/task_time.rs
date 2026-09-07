use chrono::{DateTime, Duration, Utc};
use oziclock_domain::{Planner, PlannerId, TaskStatus};

pub fn reconcile(planner: &mut Planner, now: DateTime<Utc>) -> Vec<PlannerId> {
    let mut delivered = Vec::new();
    for task in &mut planner.tasks {
        if task.status != TaskStatus::Open || task.attention_pending {
            continue;
        }
        let Some(due_text) = task.due_utc.as_deref() else {
            continue;
        };
        let Some(alert) = task.alerts.first() else {
            continue;
        };
        if task.delivered_for_due_utc.as_deref() == Some(due_text) {
            continue;
        }
        let Ok(due) = DateTime::parse_from_rfc3339(due_text) else {
            continue;
        };
        let alert_at = due.with_timezone(&Utc) - Duration::minutes(i64::from(alert.offset_minutes));
        if alert_at <= now {
            task.attention_pending = true;
            task.attention_due_utc = Some(due_text.to_owned());
            task.delivered_for_due_utc = Some(due_text.to_owned());
            delivered.push(task.id.clone());
        }
    }
    delivered
}

pub fn pending_attention(planner: &Planner) -> Vec<PlannerId> {
    let mut tasks = planner
        .tasks
        .iter()
        .filter(|task| task.attention_pending)
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.attention_due_utc.cmp(&right.attention_due_utc));
    tasks.into_iter().map(|task| task.id.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oziclock_domain::{AlertRule, Task};

    fn task() -> Task {
        Task {
            id: PlannerId::new("task").unwrap(),
            title: "Submit report".into(),
            status: TaskStatus::Open,
            due_utc: Some("2026-09-07T12:00:00+00:00".into()),
            scheduled_start_utc: None,
            scheduled_end_utc: None,
            color: None,
            tags: vec![],
            notes: None,
            alerts: vec![AlertRule {
                id: PlannerId::new("alert").unwrap(),
                offset_minutes: 5,
                play_sound: false,
            }],
            attention_pending: false,
            attention_due_utc: None,
            delivered_for_due_utc: None,
        }
    }

    #[test]
    fn scheduled_task_is_delivered_only_once_for_its_due_time() {
        let mut planner = Planner {
            tasks: vec![task()],
            ..Planner::default()
        };
        let now = DateTime::parse_from_rfc3339("2026-09-07T11:55:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(reconcile(&mut planner, now).len(), 1);
        planner.tasks[0].dismiss_attention();
        assert!(reconcile(&mut planner, now + Duration::minutes(1)).is_empty());
    }
}
