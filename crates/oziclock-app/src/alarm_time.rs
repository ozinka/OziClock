//! Wall-clock alarm resolution, independent of the system clock.
use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use oziclock_domain::{Alarm, AlarmOccurrenceStatus, AlarmReceipt, AlarmSchedule, Planner};

/// Re-enabling a Once alarm schedules a fresh occurrence in its saved zone.
pub fn set_enabled(alarm: &mut Alarm, enabled: bool, now: DateTime<Utc>) -> bool {
    if enabled
        && !alarm.enabled
        && let AlarmSchedule::Once { local_time, .. } = &alarm.schedule
    {
        let Some((date, _)) = next_once(local_time, &alarm.time_zone, now) else {
            return false;
        };
        alarm.schedule = AlarmSchedule::Once {
            local_date: date.to_string(),
            local_time: local_time.clone(),
        };
    }
    alarm.set_enabled(enabled);
    true
}

/// Resolve gaps to the first valid minute and overlaps to the earlier instant.
fn resolve(date: NaiveDate, time: NaiveTime, zone: Tz) -> Option<DateTime<Utc>> {
    let mut local = date.and_time(time);
    // Also covers historical full-day time-zone jumps.
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

/// Return the requested local date and its next strictly future occurrence.
pub fn next_once(time: &str, zone: &str, now: DateTime<Utc>) -> Option<(NaiveDate, DateTime<Utc>)> {
    let time = NaiveTime::parse_from_str(time, "%H:%M").ok()?;
    let zone: Tz = zone.parse().ok()?;
    let today = now.with_timezone(&zone).date_naive();
    for offset in 0..=2 {
        let date = today.checked_add_signed(Duration::days(offset))?;
        let instant = resolve(date, time, zone)?;
        if instant > now {
            return Some((date, instant));
        }
    }
    None
}

/// Return the next strictly future weekly occurrence.
pub fn next_weekly(
    time: &str,
    weekdays: &[u8],
    zone: &str,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    use chrono::Datelike;
    let time = NaiveTime::parse_from_str(time, "%H:%M").ok()?;
    let zone: Tz = zone.parse().ok()?;
    let today = now.with_timezone(&zone).date_naive();
    for offset in 0..=7 {
        let date = today.checked_add_signed(Duration::days(offset))?;
        if !weekdays.contains(&(date.weekday().num_days_from_monday() as u8)) {
            continue;
        }
        let instant = resolve(date, time, zone)?;
        if instant > now {
            return Some(instant);
        }
    }
    None
}

pub fn next_occurrence(alarm: &Alarm, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if !alarm.enabled {
        return None;
    }
    match &alarm.schedule {
        AlarmSchedule::Once {
            local_date,
            local_time,
        } => {
            let date = NaiveDate::parse_from_str(local_date, "%Y-%m-%d").ok()?;
            let time = NaiveTime::parse_from_str(local_time, "%H:%M").ok()?;
            let zone: Tz = alarm.time_zone.parse().ok()?;
            let instant = resolve(date, time, zone)?;
            (instant > now).then_some(instant)
        }
        AlarmSchedule::Weekly {
            local_time,
            weekdays,
        } => next_weekly(local_time, weekdays, &alarm.time_zone, now),
    }
}

/// Return enabled alarm occurrences in `(after, through]` without mutating storage.
pub fn due_between(
    alarms: &[Alarm],
    after: DateTime<Utc>,
    through: DateTime<Utc>,
) -> Vec<(oziclock_domain::PlannerId, DateTime<Utc>)> {
    if through <= after {
        return vec![];
    }
    let mut due = alarms
        .iter()
        .filter_map(|alarm| {
            next_occurrence(alarm, after)
                .filter(|occurrence| *occurrence <= through)
                .map(|occurrence| (alarm.id.clone(), occurrence))
        })
        .collect::<Vec<_>>();
    due.sort_by_key(|(_, occurrence)| *occurrence);
    due
}

/// Record each occurrence once and disable handled one-time alarms.
pub fn reconcile(
    planner: &mut Planner,
    after: DateTime<Utc>,
    through: DateTime<Utc>,
    delivery_grace: Duration,
) -> Vec<AlarmReceipt> {
    let due = due_between(&planner.alarms, after, through);
    let mut added = vec![];
    for (alarm_id, occurrence) in due {
        let occurrence_utc = occurrence.to_rfc3339();
        if planner
            .alarm_receipts
            .iter()
            .any(|receipt| receipt.alarm_id == alarm_id && receipt.occurrence_utc == occurrence_utc)
        {
            continue;
        }
        let status = if through - occurrence <= delivery_grace {
            AlarmOccurrenceStatus::Delivered
        } else {
            AlarmOccurrenceStatus::Missed
        };
        let receipt = AlarmReceipt {
            alarm_id: alarm_id.clone(),
            occurrence_utc,
            status,
            recorded_at_utc: through.to_rfc3339(),
        };
        if let Some(alarm) = planner.alarms.iter_mut().find(|alarm| alarm.id == alarm_id)
            && matches!(alarm.schedule, AlarmSchedule::Once { .. })
        {
            alarm.enabled = false;
        }
        planner.alarm_receipts.push(receipt.clone());
        added.push(receipt);
    }
    planner.alarm_checked_at_utc = Some(through.to_rfc3339());
    added
}

#[cfg(test)]
mod tests {
    use super::*;
    fn utc(value: &str) -> DateTime<Utc> {
        value.parse().unwrap()
    }

    #[test]
    fn chooses_today_or_tomorrow_and_rolls_over_year() {
        let now = utc("2026-12-31T10:00:00Z");
        assert_eq!(
            next_once("11:00", "UTC", now).unwrap().1,
            utc("2026-12-31T11:00:00Z")
        );
        assert_eq!(
            next_once("10:00", "UTC", now).unwrap().1,
            utc("2027-01-01T10:00:00Z")
        );
        assert_eq!(
            next_once("09:00", "UTC", now).unwrap().1,
            utc("2027-01-01T09:00:00Z")
        );
    }

    #[test]
    fn uses_alarm_zone_not_utc_date() {
        let (date, instant) =
            next_once("08:00", "Asia/Tokyo", utc("2026-09-04T23:30:00Z")).unwrap();
        assert_eq!(date.to_string(), "2026-09-06");
        assert_eq!(instant, utc("2026-09-05T23:00:00Z"));
    }

    #[test]
    fn dst_gap_uses_first_valid_minute() {
        assert_eq!(
            next_once("02:30", "Europe/Berlin", utc("2026-03-29T00:00:00Z"))
                .unwrap()
                .1,
            utc("2026-03-29T01:00:00Z")
        );
    }

    #[test]
    fn dst_overlap_uses_earlier_occurrence_only() {
        assert_eq!(
            next_once("02:30", "Europe/Berlin", utc("2026-10-25T00:00:00Z"))
                .unwrap()
                .1,
            utc("2026-10-25T00:30:00Z")
        );
        assert_eq!(
            next_once("02:30", "Europe/Berlin", utc("2026-10-25T00:45:00Z"))
                .unwrap()
                .1,
            utc("2026-10-26T01:30:00Z")
        );
    }

    #[test]
    fn rejects_invalid_inputs() {
        let now = utc("2026-09-04T00:00:00Z");
        assert!(next_once("25:00", "UTC", now).is_none());
        assert!(next_once("12:00", "invalid", now).is_none());
    }

    #[test]
    fn weekly_skips_unselected_days_and_advances_after_time() {
        let friday = utc("2026-09-04T12:00:00Z");
        assert_eq!(
            next_weekly("09:00", &[0, 4], "UTC", friday).unwrap(),
            utc("2026-09-07T09:00:00Z")
        );
        assert_eq!(
            next_weekly("13:00", &[0, 4], "UTC", friday).unwrap(),
            utc("2026-09-04T13:00:00Z")
        );
        assert!(next_weekly("13:00", &[], "UTC", friday).is_none());
    }

    #[test]
    fn occurrence_ignores_disabled_and_expired_once_alarm() {
        let mut alarm = Alarm {
            id: oziclock_domain::PlannerId::new("a").unwrap(),
            title: "Alarm".into(),
            enabled: false,
            time_zone: "UTC".into(),
            schedule: AlarmSchedule::Once {
                local_date: "2026-09-05".into(),
                local_time: "09:00".into(),
            },
        };
        let now = utc("2026-09-05T08:00:00Z");
        assert!(next_occurrence(&alarm, now).is_none());
        alarm.enabled = true;
        assert_eq!(
            next_occurrence(&alarm, now).unwrap(),
            utc("2026-09-05T09:00:00Z")
        );
        assert!(next_occurrence(&alarm, utc("2026-09-05T10:00:00Z")).is_none());
    }

    #[test]
    fn due_interval_is_open_then_closed_and_sorted() {
        let alarm = |id: &str, time: &str| Alarm {
            id: oziclock_domain::PlannerId::new(id).unwrap(),
            title: id.into(),
            enabled: true,
            time_zone: "UTC".into(),
            schedule: AlarmSchedule::Once {
                local_date: "2026-09-05".into(),
                local_time: time.into(),
            },
        };
        let alarms = vec![
            alarm("later", "09:02"),
            alarm("first", "09:01"),
            alarm("boundary", "09:00"),
        ];
        let due = due_between(
            &alarms,
            utc("2026-09-05T09:00:00Z"),
            utc("2026-09-05T09:02:00Z"),
        );
        assert_eq!(
            due.iter().map(|(id, _)| id.to_string()).collect::<Vec<_>>(),
            ["first", "later"]
        );
        assert!(
            due_between(
                &alarms,
                utc("2026-09-05T09:02:00Z"),
                utc("2026-09-05T09:02:00Z")
            )
            .is_empty()
        );
    }

    #[test]
    fn reconciliation_records_once_disables_once_and_prevents_duplicates() {
        let mut planner = Planner::default();
        planner.alarms.push(Alarm {
            id: oziclock_domain::PlannerId::new("a").unwrap(),
            title: "Alarm".into(),
            enabled: true,
            time_zone: "UTC".into(),
            schedule: AlarmSchedule::Once {
                local_date: "2026-09-05".into(),
                local_time: "09:01".into(),
            },
        });
        let after = utc("2026-09-05T09:00:00Z");
        let through = utc("2026-09-05T09:02:00Z");
        let added = reconcile(&mut planner, after, through, Duration::minutes(5));
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].status, AlarmOccurrenceStatus::Delivered);
        assert!(!planner.alarms[0].enabled);
        assert!(reconcile(&mut planner, after, through, Duration::minutes(5)).is_empty());
        assert_eq!(planner.alarm_receipts.len(), 1);
    }

    #[test]
    fn reconciliation_marks_occurrences_outside_grace_as_missed() {
        let mut planner = Planner::default();
        planner.alarms.push(Alarm {
            id: oziclock_domain::PlannerId::new("a").unwrap(),
            title: "Alarm".into(),
            enabled: true,
            time_zone: "UTC".into(),
            schedule: AlarmSchedule::Once {
                local_date: "2026-09-05".into(),
                local_time: "09:01".into(),
            },
        });
        let added = reconcile(
            &mut planner,
            utc("2026-09-05T09:00:00Z"),
            utc("2026-09-05T09:20:00Z"),
            Duration::minutes(5),
        );
        assert_eq!(added[0].status, AlarmOccurrenceStatus::Missed);
    }

    #[test]
    fn rearming_once_updates_date_but_disabling_does_not() {
        let mut alarm = Alarm {
            id: oziclock_domain::PlannerId::new("a").unwrap(),
            title: "Alarm".into(),
            enabled: true,
            time_zone: "UTC".into(),
            schedule: AlarmSchedule::Once {
                local_date: "2020-01-01".into(),
                local_time: "07:00".into(),
            },
        };
        let original = alarm.schedule.clone();
        let now = utc("2026-09-04T08:00:00Z");
        assert!(set_enabled(&mut alarm, false, now));
        assert_eq!(alarm.schedule, original);
        assert!(set_enabled(&mut alarm, true, now));
        assert_eq!(
            alarm.schedule,
            AlarmSchedule::Once {
                local_date: "2026-09-05".into(),
                local_time: "07:00".into()
            }
        );
        assert!(alarm.enabled);
    }
}
