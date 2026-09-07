use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use oziclock_domain::{Event, EventReceipt, EventRecurrence, EventTime, Planner};

fn recurrence_matches(origin: NaiveDate, date: NaiveDate, recurrence: EventRecurrence) -> bool {
    if date < origin {
        return false;
    }
    match recurrence {
        EventRecurrence::None => date == origin,
        EventRecurrence::Daily => true,
        EventRecurrence::Weekly => date.weekday() == origin.weekday(),
        EventRecurrence::Monthly => date.day() == origin.day(),
        EventRecurrence::Yearly => date.month() == origin.month() && date.day() == origin.day(),
    }
}

fn starts_between(
    event: &Event,
    after: DateTime<Utc>,
    now: DateTime<Utc>,
    all_day_zone: &str,
) -> Vec<DateTime<Utc>> {
    let (origin, local_time, zone_name) = match &event.time {
        EventTime::Timed {
            start_utc,
            source_time_zone,
            ..
        } => {
            let Ok(start) = DateTime::parse_from_rfc3339(start_utc) else {
                return vec![];
            };
            let Ok(zone) = source_time_zone.parse::<Tz>() else {
                return vec![];
            };
            let local = start.with_timezone(&zone);
            (local.date_naive(), local.time(), source_time_zone.as_str())
        }
        EventTime::AllDay { start_date, .. } => {
            let Ok(date) = NaiveDate::parse_from_str(start_date, "%Y-%m-%d") else {
                return vec![];
            };
            (date, chrono::NaiveTime::MIN, all_day_zone)
        }
    };
    let Ok(zone) = zone_name.parse::<Tz>() else {
        return vec![];
    };
    let maximum_offset = event
        .alerts
        .iter()
        .map(|alert| alert.offset_minutes.max(0))
        .max()
        .unwrap_or_default();
    let local_from = (after - Duration::days(1))
        .with_timezone(&zone)
        .date_naive();
    let local_to = (now + Duration::minutes(i64::from(maximum_offset)) + Duration::days(1))
        .with_timezone(&zone)
        .date_naive();
    (0..=local_to.signed_duration_since(local_from).num_days())
        .map(|offset| local_from + Duration::days(offset))
        .filter(|date| recurrence_matches(origin, *date, event.recurrence))
        .filter_map(|date| {
            let local = date.and_time(local_time);
            match zone.from_local_datetime(&local) {
                chrono::LocalResult::Single(value) => Some(value.with_timezone(&Utc)),
                chrono::LocalResult::Ambiguous(first, second) => {
                    Some(first.min(second).with_timezone(&Utc))
                }
                chrono::LocalResult::None => None,
            }
        })
        .collect()
}

pub fn reconcile(
    planner: &mut Planner,
    after: DateTime<Utc>,
    now: DateTime<Utc>,
    all_day_zone: &str,
) -> Vec<EventReceipt> {
    let mut delivered = Vec::new();
    for event in &planner.events {
        for occurrence in starts_between(event, after, now, all_day_zone) {
            for alert in &event.alerts {
                let due = occurrence - Duration::minutes(i64::from(alert.offset_minutes.max(0)));
                if due <= after || due > now {
                    continue;
                }
                let occurrence_utc = occurrence.to_rfc3339();
                if planner.event_receipts.iter().any(|receipt| {
                    receipt.event_id == event.id
                        && receipt.occurrence_utc == occurrence_utc
                        && receipt.alert_offset_minutes == alert.offset_minutes
                }) {
                    continue;
                }
                delivered.push(EventReceipt {
                    event_id: event.id.clone(),
                    occurrence_utc,
                    alert_offset_minutes: alert.offset_minutes,
                    delivered_at_utc: now.to_rfc3339(),
                    acknowledged_at_utc: None,
                });
            }
        }
    }
    delivered.sort_by(|left, right| left.occurrence_utc.cmp(&right.occurrence_utc));
    planner.event_receipts.extend(delivered.iter().cloned());
    planner.event_checked_at_utc = Some(now.to_rfc3339());
    delivered
}

pub fn pending_attention(planner: &Planner) -> Vec<EventReceipt> {
    let mut pending = planner
        .event_receipts
        .iter()
        .filter(|receipt| receipt.acknowledged_at_utc.is_none())
        .cloned()
        .collect::<Vec<_>>();
    pending.sort_by(|left, right| left.delivered_at_utc.cmp(&right.delivered_at_utc));
    pending
}

#[cfg(test)]
mod tests {
    use super::*;
    use oziclock_domain::{AlertRule, Event, PlannerId};

    fn id(value: &str) -> PlannerId {
        PlannerId::new(value).unwrap()
    }

    #[test]
    fn recurring_event_alert_is_delivered_once_and_restored_until_acknowledged() {
        let mut planner = Planner::default();
        planner.events.push(Event {
            id: id("standup"),
            title: "Standup".into(),
            time: EventTime::Timed {
                start_utc: "2026-09-07T09:00:00Z".into(),
                end_utc: "2026-09-07T09:30:00Z".into(),
                source_time_zone: "UTC".into(),
            },
            location: None,
            notes: None,
            link: None,
            color: None,
            alerts: vec![AlertRule {
                id: id("alert"),
                offset_minutes: 15,
                play_sound: false,
            }],
            recurrence: EventRecurrence::Daily,
        });
        let after = "2026-09-08T08:44:00Z".parse().unwrap();
        let now = "2026-09-08T08:45:00Z".parse().unwrap();
        assert_eq!(reconcile(&mut planner, after, now, "UTC").len(), 1);
        assert!(reconcile(&mut planner, after, now, "UTC").is_empty());
        assert_eq!(pending_attention(&planner).len(), 1);
        planner.event_receipts[0].acknowledged_at_utc = Some(now.to_rfc3339());
        assert!(pending_attention(&planner).is_empty());
    }
}
