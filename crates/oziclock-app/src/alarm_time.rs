//! Wall-clock alarm resolution, independent of the system clock.
use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use oziclock_domain::{Alarm, AlarmSchedule};

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
