use chrono::{DateTime, Utc};
use oziclock_storage::{AppSettings, EventReceipt};

pub(super) fn reconcile_tick<E>(
    settings: &mut AppSettings,
    now: DateTime<Utc>,
    zone: &str,
    persist: impl FnOnce(&AppSettings) -> Result<(), E>,
) -> Result<Option<Vec<EventReceipt>>, E> {
    let saved_after = settings
        .planner
        .event_checked_at_utc
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc));
    let after = saved_after.unwrap_or(now - chrono::Duration::seconds(1));
    if saved_after.is_none() {
        settings.planner.event_checked_at_utc = Some(after.to_rfc3339());
    }
    let after = after.max(now - chrono::Duration::minutes(5));
    let mut updated = settings.clone();
    let receipts = oziclock_app::event_time::reconcile(&mut updated.planner, after, now, zone);
    oziclock_storage::prune_event_receipts(&mut updated, now);
    if settings.planner.event_receipts == updated.planner.event_receipts {
        settings.planner.event_checked_at_utc = updated.planner.event_checked_at_utc;
        return Ok(None);
    }
    persist(&updated)?;
    *settings = updated;
    Ok(Some(receipts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oziclock_storage::{AlertRule, Event, EventRecurrence, EventTime, PlannerId};

    fn utc(value: &str) -> DateTime<Utc> {
        value.parse().unwrap()
    }
    fn settings() -> AppSettings {
        let mut settings: AppSettings = serde_json::from_str(include_str!(
            "../../../../crates/oziclock-storage/assets/default_settings.json"
        ))
        .unwrap();
        settings.planner.event_checked_at_utc = Some("2026-10-13T09:00:00Z".into());
        settings.planner.events.push(Event {
            id: PlannerId::new("event").unwrap(),
            title: "Test".into(),
            time: EventTime::Timed {
                start_utc: "2026-10-13T10:00:00Z".into(),
                end_utc: "2026-10-13T11:00:00Z".into(),
                source_time_zone: "UTC".into(),
            },
            location: None,
            notes: None,
            link: None,
            color: None,
            alerts: vec![AlertRule {
                id: PlannerId::new("alert").unwrap(),
                offset_minutes: 0,
                play_sound: false,
            }],
            recurrence: EventRecurrence::None,
        });
        settings
    }

    #[test]
    fn nfr_04a_idle_hour_performs_zero_writes() {
        let mut state = settings();
        let start = utc("2026-10-13T09:00:00Z");
        for second in 1..3600 {
            assert!(
                reconcile_tick::<()>(
                    &mut state,
                    start + chrono::Duration::seconds(second),
                    "UTC",
                    |_| panic!("idle write")
                )
                .unwrap()
                .is_none()
            );
        }
        assert_eq!(
            state.planner.event_checked_at_utc,
            Some(utc("2026-10-13T09:59:59Z").to_rfc3339())
        );
    }

    #[test]
    fn evt_10_failed_save_retries_before_delivery() {
        for cursor in [
            None,
            Some("invalid".to_owned()),
            Some("2026-10-13T09:59:59Z".to_owned()),
        ] {
            let mut state = settings();
            state.planner.event_checked_at_utc = cursor;
            assert!(
                reconcile_tick(&mut state, utc("2026-10-13T10:00:00Z"), "UTC", |_| Err(()))
                    .is_err()
            );
            assert!(state.planner.event_receipts.is_empty());
            let result =
                reconcile_tick::<()>(&mut state, utc("2026-10-13T10:00:01Z"), "UTC", |saved| {
                    assert_eq!(saved.planner.event_receipts.len(), 1);
                    Ok(())
                })
                .unwrap()
                .unwrap();
            assert_eq!(result.len(), 1);
            assert!(
                reconcile_tick::<()>(&mut state, utc("2026-10-13T10:00:02Z"), "UTC", |_| panic!(
                    "duplicate write"
                ))
                .unwrap()
                .is_none()
            );
        }
    }

    #[test]
    fn evt_11_restart_recovers_within_five_minutes_without_duplicates() {
        let mut state = settings();
        let mut saved = state.clone();
        reconcile_tick::<()>(&mut state, utc("2026-10-13T09:59:59Z"), "UTC", |_| {
            panic!("idle write")
        })
        .unwrap();
        let receipts =
            reconcile_tick::<()>(&mut saved, utc("2026-10-13T10:01:00Z"), "UTC", |_| Ok(()))
                .unwrap()
                .unwrap();
        assert_eq!(receipts.len(), 1);
        assert!(
            reconcile_tick::<()>(&mut saved, utc("2026-10-13T10:02:00Z"), "UTC", |_| panic!(
                "duplicate write"
            ))
            .unwrap()
            .is_none()
        );
        assert_eq!(
            oziclock_app::event_time::pending_attention(&saved.planner).len(),
            1
        );
        let mut late = settings();
        assert!(
            reconcile_tick::<()>(&mut late, utc("2026-10-13T10:06:00Z"), "UTC", |_| panic!(
                "late write"
            ))
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn evt_11_receipt_pruning_requires_successful_save() {
        let mut state = settings();
        state.planner.event_receipts.push(EventReceipt {
            event_id: PlannerId::new("event").unwrap(),
            occurrence_utc: "2026-09-01T10:00:00Z".into(),
            alert_offset_minutes: 0,
            delivered_at_utc: "2026-09-01T10:00:00Z".into(),
            acknowledged_at_utc: Some("2026-09-01T10:01:00Z".into()),
        });
        assert!(
            reconcile_tick(&mut state, utc("2026-10-13T09:00:01Z"), "UTC", |_| Err(())).is_err()
        );
        assert_eq!(state.planner.event_receipts.len(), 1);
        assert!(
            reconcile_tick::<()>(&mut state, utc("2026-10-13T09:00:02Z"), "UTC", |_| Ok(()))
                .unwrap()
                .unwrap()
                .is_empty()
        );
        assert!(state.planner.event_receipts.is_empty());
    }
}
