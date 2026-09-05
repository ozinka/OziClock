//! Deterministic one-time reminder delivery and restart recovery.

use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use oziclock_domain::{Planner, PlannerId, ReminderSchedule};

pub fn due_utc(schedule: &ReminderSchedule) -> Option<DateTime<Utc>> {
    match schedule {
        ReminderSchedule::Absolute { at_utc, .. } => DateTime::parse_from_rfc3339(at_utc)
            .ok()
            .map(|value| value.with_timezone(&Utc)),
        ReminderSchedule::ImportantDate { .. } => None,
    }
}

pub fn resolve_local(date: &str, time: &str, zone: &str) -> Option<DateTime<Utc>> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let time = NaiveTime::parse_from_str(time, "%H:%M").ok()?;
    let zone: Tz = zone.parse().ok()?;
    let mut local = date.and_time(time);
    for _ in 0..=1440 {
        match zone.from_local_datetime(&local) {
            LocalResult::Single(value) => return Some(value.with_timezone(&Utc)),
            LocalResult::Ambiguous(first, second) => {
                return Some(first.min(second).with_timezone(&Utc));
            }
            LocalResult::None => local = local.checked_add_signed(Duration::minutes(1))?,
        }
    }
    None
}

pub fn reconcile(planner: &mut Planner, now: DateTime<Utc>) -> Vec<PlannerId> {
    let mut due = planner
        .reminders
        .iter()
        .filter_map(|reminder| {
            let due = due_utc(&reminder.schedule)?;
            (reminder.enabled && due <= now).then(|| (due, reminder.id.clone()))
        })
        .collect::<Vec<_>>();
    due.sort_by_key(|(due, _)| *due);
    due.into_iter()
        .map(|(_, id)| id)
        .filter(|id| {
            planner
                .reminders
                .iter_mut()
                .find(|reminder| &reminder.id == id)
                .is_some_and(|reminder| reminder.deliver())
        })
        .collect()
}

pub fn pending_attention(planner: &Planner) -> Vec<PlannerId> {
    let mut pending = planner
        .reminders
        .iter()
        .filter(|reminder| reminder.attention_pending)
        .map(|reminder| (due_utc(&reminder.schedule), reminder.id.clone()))
        .collect::<Vec<_>>();
    pending.sort_by_key(|(due, _)| *due);
    pending.into_iter().map(|(_, id)| id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oziclock_domain::Reminder;

    fn reminder(at: &str) -> Reminder {
        Reminder {
            id: PlannerId::new("reminder").unwrap(),
            title: "Stretch".into(),
            schedule: ReminderSchedule::Absolute {
                at_utc: at.into(),
                source_time_zone: "UTC".into(),
            },
            alerts: vec![],
            enabled: true,
            attention_pending: false,
        }
    }

    #[test]
    fn due_reminder_is_delivered_once_and_restored_until_dismissed() {
        let mut planner = Planner {
            reminders: vec![reminder("2026-09-05T10:00:00Z")],
            ..Planner::default()
        };
        let now = "2026-09-05T10:00:01Z".parse().unwrap();
        assert_eq!(reconcile(&mut planner, now).len(), 1);
        assert!(!planner.reminders[0].enabled);
        assert_eq!(pending_attention(&planner).len(), 1);
        assert!(reconcile(&mut planner, now).is_empty());
    }

    #[test]
    fn invalid_and_future_reminders_do_not_fire() {
        let now = "2026-09-05T10:00:00Z".parse().unwrap();
        for at in ["invalid", "2026-09-05T10:00:01Z"] {
            let mut planner = Planner {
                reminders: vec![reminder(at)],
                ..Planner::default()
            };
            assert!(reconcile(&mut planner, now).is_empty());
        }
    }

    #[test]
    fn resolves_local_date_and_time_in_the_selected_zone() {
        assert_eq!(
            resolve_local("2026-09-05", "12:30", "Europe/Kyiv")
                .unwrap()
                .to_rfc3339(),
            "2026-09-05T09:30:00+00:00"
        );
        assert!(resolve_local("invalid", "12:30", "UTC").is_none());
        assert!(resolve_local("2026-09-05", "25:00", "UTC").is_none());
    }
}
