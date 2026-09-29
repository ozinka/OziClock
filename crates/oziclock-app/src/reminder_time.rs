//! Deterministic reminder recurrence, delivery, and restart recovery.

use chrono::{
    DateTime, Datelike, Duration, LocalResult, Months, NaiveDate, NaiveTime, TimeZone, Utc,
};
use chrono_tz::Tz;
use oziclock_domain::{Planner, PlannerId, ReminderRecurrence, ReminderSchedule};

pub fn due_utc(schedule: &ReminderSchedule) -> Option<DateTime<Utc>> {
    match schedule {
        ReminderSchedule::Absolute { at_utc, .. } => DateTime::parse_from_rfc3339(at_utc)
            .ok()
            .map(|value| value.with_timezone(&Utc)),
        ReminderSchedule::Recurring { next_at_utc, .. } => {
            DateTime::parse_from_rfc3339(next_at_utc)
                .ok()
                .map(|value| value.with_timezone(&Utc))
        }
        ReminderSchedule::ImportantDate { .. } => None,
    }
}

/// Project a saved reminder onto a date in its source time zone.
pub fn occurrence_on_date(schedule: &ReminderSchedule, date: NaiveDate) -> Option<DateTime<Utc>> {
    match schedule {
        ReminderSchedule::Absolute {
            source_time_zone, ..
        } => {
            let zone: Tz = source_time_zone.parse().ok()?;
            let due = due_utc(schedule)?;
            (due.with_timezone(&zone).date_naive() == date).then_some(due)
        }
        ReminderSchedule::Recurring {
            recurrence,
            local_time,
            source_time_zone,
            next_at_utc,
        } => {
            let zone: Tz = source_time_zone.parse().ok()?;
            let next = DateTime::parse_from_rfc3339(next_at_utc)
                .ok()?
                .with_timezone(&Utc);
            let anchor = next.with_timezone(&zone).date_naive();
            if date < anchor || !recurrence_matches_date(recurrence, date) {
                return None;
            }
            let due = resolve_local(
                &date.format("%Y-%m-%d").to_string(),
                local_time,
                source_time_zone,
            )?;
            (due >= next).then_some(due)
        }
        ReminderSchedule::ImportantDate { .. } => None,
    }
}

fn recurrence_matches_date(recurrence: &ReminderRecurrence, date: NaiveDate) -> bool {
    match recurrence {
        ReminderRecurrence::Daily => true,
        ReminderRecurrence::Weekly { weekdays } => {
            weekdays.contains(&(date.weekday().num_days_from_monday() as u8))
        }
        ReminderRecurrence::Monthly { day } => {
            clamped_date(date.year(), date.month(), *day) == Some(date)
        }
        ReminderRecurrence::MonthlyFirst => date.day() == 1,
        ReminderRecurrence::MonthlyLast => {
            clamped_date(date.year(), date.month(), 31) == Some(date)
        }
        ReminderRecurrence::Yearly { month, day } => {
            clamped_date(date.year(), u32::from(*month), *day) == Some(date)
        }
    }
}

pub fn recurring_schedule(
    recurrence: ReminderRecurrence,
    first_date: &str,
    time: &str,
    zone: &str,
) -> Option<ReminderSchedule> {
    let mut first_date = NaiveDate::parse_from_str(first_date, "%Y-%m-%d").ok()?;
    if let ReminderRecurrence::Weekly { weekdays } = &recurrence {
        first_date = (0..7).find_map(|offset| {
            let date = first_date.checked_add_signed(Duration::days(offset))?;
            weekdays
                .contains(&(date.weekday().num_days_from_monday() as u8))
                .then_some(date)
        })?;
    }
    if matches!(
        recurrence,
        ReminderRecurrence::MonthlyFirst | ReminderRecurrence::MonthlyLast
    ) {
        let day = if matches!(recurrence, ReminderRecurrence::MonthlyFirst) {
            1
        } else {
            31
        };
        let current = clamped_date(first_date.year(), first_date.month(), day)?;
        first_date = if current >= first_date {
            current
        } else {
            let next_month = first_date.with_day(1)?.checked_add_months(Months::new(1))?;
            clamped_date(next_month.year(), next_month.month(), day)?
        };
    }
    let first = resolve_local(&first_date.format("%Y-%m-%d").to_string(), time, zone)?;
    Some(ReminderSchedule::Recurring {
        recurrence,
        local_time: time.to_owned(),
        source_time_zone: zone.to_owned(),
        next_at_utc: first.to_rfc3339(),
    })
}

fn next_occurrence(schedule: &ReminderSchedule) -> Option<DateTime<Utc>> {
    let ReminderSchedule::Recurring {
        recurrence,
        local_time,
        source_time_zone,
        next_at_utc,
    } = schedule
    else {
        return None;
    };
    let zone: Tz = source_time_zone.parse().ok()?;
    let current = DateTime::parse_from_rfc3339(next_at_utc)
        .ok()?
        .with_timezone(&zone);
    let time = NaiveTime::parse_from_str(local_time, "%H:%M").ok()?;
    let date = match recurrence {
        ReminderRecurrence::Daily => current.date_naive().succ_opt()?,
        ReminderRecurrence::Weekly { weekdays } => (1..=7).find_map(|offset| {
            let date = current
                .date_naive()
                .checked_add_signed(Duration::days(offset))?;
            weekdays
                .contains(&(date.weekday().num_days_from_monday() as u8))
                .then_some(date)
        })?,
        ReminderRecurrence::Monthly { day } => {
            let month = current
                .date_naive()
                .with_day(1)?
                .checked_add_months(Months::new(1))?;
            clamped_date(month.year(), month.month(), *day)?
        }
        ReminderRecurrence::MonthlyFirst | ReminderRecurrence::MonthlyLast => {
            let month = current
                .date_naive()
                .with_day(1)?
                .checked_add_months(Months::new(1))?;
            let day = if matches!(recurrence, ReminderRecurrence::MonthlyFirst) {
                1
            } else {
                31
            };
            clamped_date(month.year(), month.month(), day)?
        }
        ReminderRecurrence::Yearly { month, day } => {
            clamped_date(current.year() + 1, u32::from(*month), *day)?
        }
    };
    resolve_local(
        &date.format("%Y-%m-%d").to_string(),
        &time.format("%H:%M").to_string(),
        source_time_zone,
    )
}

fn clamped_date(year: i32, month: u32, preferred_day: u8) -> Option<NaiveDate> {
    (1..=u32::from(preferred_day))
        .rev()
        .find_map(|day| NaiveDate::from_ymd_opt(year, month, day))
}

fn advance_past(schedule: &mut ReminderSchedule, now: DateTime<Utc>) {
    for _ in 0..10_000 {
        let Some(next) = next_occurrence(schedule) else {
            return;
        };
        if let ReminderSchedule::Recurring { next_at_utc, .. } = schedule {
            *next_at_utc = next.to_rfc3339();
        }
        if next > now {
            return;
        }
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
    for reminder in planner
        .reminders
        .iter_mut()
        .filter(|reminder| reminder.attention_pending)
    {
        if due_utc(&reminder.schedule).is_some_and(|due| due <= now) {
            advance_past(&mut reminder.schedule, now);
        }
    }
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
        .filter_map(|(due, id)| {
            let delivered = planner
                .reminders
                .iter_mut()
                .find(|reminder| reminder.id == id)
                .is_some_and(|reminder| {
                    let delivered = reminder.deliver(due.to_rfc3339());
                    if delivered {
                        advance_past(&mut reminder.schedule, now);
                    }
                    delivered
                });
            delivered.then_some(id)
        })
        .collect()
}

pub fn pending_attention(planner: &Planner) -> Vec<PlannerId> {
    let mut pending = planner
        .reminders
        .iter()
        .filter(|reminder| reminder.attention_pending)
        .map(|reminder| {
            (
                reminder
                    .attention_due_utc
                    .as_deref()
                    .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                    .map(|value| value.with_timezone(&Utc)),
                reminder.id.clone(),
            )
        })
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
            attention_due_utc: None,
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

    #[test]
    fn pln_02_monthly_projection_shows_later_month_and_clamps_day() {
        let schedule = recurring_schedule(
            ReminderRecurrence::Monthly { day: 31 },
            "2026-01-31",
            "10:00",
            "Europe/Kyiv",
        )
        .unwrap();
        let date = |month, day| NaiveDate::from_ymd_opt(2026, month, day).unwrap();
        assert!(occurrence_on_date(&schedule, date(1, 30)).is_none());
        assert_eq!(
            occurrence_on_date(&schedule, date(2, 28))
                .unwrap()
                .to_rfc3339(),
            "2026-02-28T08:00:00+00:00"
        );
        assert_eq!(
            occurrence_on_date(&schedule, date(3, 31))
                .unwrap()
                .to_rfc3339(),
            "2026-03-31T07:00:00+00:00"
        );
    }

    #[test]
    fn pln_02_projection_starts_at_saved_next_occurrence() {
        let schedule =
            recurring_schedule(ReminderRecurrence::Daily, "2026-09-05", "10:00", "UTC").unwrap();
        let date = |day| NaiveDate::from_ymd_opt(2026, 9, day).unwrap();
        assert!(occurrence_on_date(&schedule, date(4)).is_none());
        assert!(occurrence_on_date(&schedule, date(5)).is_some());
        assert!(occurrence_on_date(&schedule, date(6)).is_some());
    }

    #[test]
    fn rem_16_first_and_last_day_follow_calendar_months() {
        let first = recurring_schedule(
            ReminderRecurrence::MonthlyFirst,
            "2027-01-15",
            "09:00",
            "UTC",
        )
        .unwrap();
        assert_eq!(
            due_utc(&first).unwrap().date_naive().to_string(),
            "2027-02-01"
        );
        assert_eq!(
            next_occurrence(&first).unwrap().date_naive().to_string(),
            "2027-03-01"
        );

        let last = recurring_schedule(
            ReminderRecurrence::MonthlyLast,
            "2027-01-15",
            "09:00",
            "UTC",
        )
        .unwrap();
        assert_eq!(
            due_utc(&last).unwrap().date_naive().to_string(),
            "2027-01-31"
        );
        assert_eq!(
            next_occurrence(&last).unwrap().date_naive().to_string(),
            "2027-02-28"
        );
        assert!(occurrence_on_date(&last, NaiveDate::from_ymd_opt(2027, 3, 31).unwrap()).is_some());
        assert!(occurrence_on_date(&last, NaiveDate::from_ymd_opt(2027, 3, 30).unwrap()).is_none());
    }

    #[test]
    fn recurring_reminder_advances_and_stays_enabled_after_delivery() {
        let schedule =
            recurring_schedule(ReminderRecurrence::Daily, "2026-09-05", "10:00", "UTC").unwrap();
        let mut reminder = reminder("2026-09-05T10:00:00Z");
        reminder.schedule = schedule;
        let mut planner = Planner {
            reminders: vec![reminder],
            ..Planner::default()
        };
        let now = "2026-09-05T10:00:01Z".parse().unwrap();
        assert_eq!(reconcile(&mut planner, now).len(), 1);
        assert!(planner.reminders[0].enabled);
        assert!(planner.reminders[0].attention_pending);
        assert_eq!(
            due_utc(&planner.reminders[0].schedule)
                .unwrap()
                .to_rfc3339(),
            "2026-09-06T10:00:00+00:00"
        );
    }

    #[test]
    fn weekly_uses_selected_days_and_monthly_clamps_then_recovers_day() {
        let weekly = recurring_schedule(
            ReminderRecurrence::Weekly {
                weekdays: vec![0, 4],
            },
            "2026-09-05",
            "09:00",
            "UTC",
        )
        .unwrap();
        assert_eq!(
            due_utc(&weekly).unwrap().date_naive().to_string(),
            "2026-09-07"
        );

        let mut monthly = recurring_schedule(
            ReminderRecurrence::Monthly { day: 31 },
            "2027-01-31",
            "09:00",
            "UTC",
        )
        .unwrap();
        let february = next_occurrence(&monthly).unwrap();
        assert_eq!(february.date_naive().to_string(), "2027-02-28");
        if let ReminderSchedule::Recurring { next_at_utc, .. } = &mut monthly {
            *next_at_utc = february.to_rfc3339();
        }
        assert_eq!(
            next_occurrence(&monthly).unwrap().date_naive().to_string(),
            "2027-03-31"
        );
    }

    #[test]
    fn yearly_february_29_clamps_in_non_leap_years() {
        let yearly = recurring_schedule(
            ReminderRecurrence::Yearly { month: 2, day: 29 },
            "2028-02-29",
            "08:00",
            "UTC",
        )
        .unwrap();
        assert_eq!(
            next_occurrence(&yearly).unwrap().date_naive().to_string(),
            "2029-02-28"
        );
    }
}
