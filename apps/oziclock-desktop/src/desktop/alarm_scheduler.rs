//! Commit durable alarm changes before delivery; keep idle checks in memory.
use chrono::{DateTime, Utc};
use oziclock_storage::{AlarmReceipt, AppSettings};

pub(super) fn reconcile_tick<E>(
    settings: &mut AppSettings,
    now: DateTime<Utc>,
    persist: impl FnOnce(&AppSettings) -> Result<(), E>,
) -> Result<Option<Vec<AlarmReceipt>>, E> {
    let saved_after = settings
        .planner
        .alarm_checked_at_utc
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc));
    let after = saved_after.unwrap_or(now - chrono::Duration::seconds(1));
    if saved_after.is_none() {
        // Keep the first interval stable even if its first durable save fails.
        settings.planner.alarm_checked_at_utc = Some(after.to_rfc3339());
    }
    let mut updated = settings.clone();
    let mut receipts = oziclock_app::alarm_time::reconcile(
        &mut updated.planner,
        after,
        now,
        chrono::Duration::minutes(5),
    );
    receipts.extend(oziclock_app::alarm_time::reconcile_snoozes(
        &mut updated.planner,
        now,
        chrono::Duration::minutes(5),
    ));
    oziclock_storage::prune_alarm_receipts(&mut updated, now);
    let changed = settings.planner.alarms != updated.planner.alarms
        || settings.planner.alarm_receipts != updated.planner.alarm_receipts
        || settings.planner.alarm_snoozes != updated.planner.alarm_snoozes;
    if !changed {
        settings.planner.alarm_checked_at_utc = updated.planner.alarm_checked_at_utc;
        return Ok(None);
    }
    persist(&updated)?;
    *settings = updated;
    Ok(Some(receipts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oziclock_storage::{Alarm, AlarmOccurrenceStatus, AlarmSchedule, AlarmSnooze, PlannerId};

    fn utc(value: &str) -> DateTime<Utc> {
        value.parse().unwrap()
    }
    fn settings() -> AppSettings {
        let mut settings: AppSettings = serde_json::from_str(include_str!(
            "../../../../crates/oziclock-storage/assets/default_settings.json"
        ))
        .unwrap();
        settings.planner.alarm_checked_at_utc = Some("2026-10-13T09:00:00Z".into());
        settings.planner.alarms.push(Alarm {
            id: PlannerId::new("test").unwrap(),
            title: "Test".into(),
            enabled: true,
            time_zone: "UTC".into(),
            schedule: AlarmSchedule::Once {
                local_date: "2026-10-13".into(),
                local_time: "10:00".into(),
            },
        });
        settings
    }

    #[test]
    fn nfr_04a_idle_hour_performs_zero_writes() {
        let mut settings = settings();
        let start = utc("2026-10-13T09:00:00Z");
        for second in 1..3600 {
            assert!(
                reconcile_tick::<()>(
                    &mut settings,
                    start + chrono::Duration::seconds(second),
                    |_| panic!("idle write")
                )
                .unwrap()
                .is_none()
            );
        }
        assert_eq!(
            settings.planner.alarm_checked_at_utc,
            Some(utc("2026-10-13T09:59:59Z").to_rfc3339())
        );
    }

    #[test]
    fn alm_13_failed_save_keeps_cursor_and_retries_before_delivery() {
        let mut settings = settings();
        let before = settings.planner.clone();
        assert!(
            reconcile_tick(&mut settings, utc("2026-10-13T10:00:00Z"), |_| Err(
                "disk full"
            ))
            .is_err()
        );
        assert_eq!(settings.planner, before);
        let receipts =
            reconcile_tick::<()>(&mut settings, utc("2026-10-13T10:00:01Z"), |updated| {
                assert!(!updated.planner.alarms[0].enabled);
                assert_eq!(updated.planner.alarm_receipts.len(), 1);
                Ok(())
            })
            .unwrap()
            .unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].status, AlarmOccurrenceStatus::Delivered);
        assert!(
            reconcile_tick::<()>(&mut settings, utc("2026-10-13T10:00:02Z"), |_| panic!(
                "duplicate write"
            ))
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn alm_13_first_check_without_cursor_retries_the_same_interval() {
        for cursor in [None, Some("invalid".to_string())] {
            let mut settings = settings();
            settings.planner.alarm_checked_at_utc = cursor;
            assert!(
                reconcile_tick(&mut settings, utc("2026-10-13T10:00:00Z"), |_| Err(())).is_err()
            );
            assert!(settings.planner.alarm_receipts.is_empty());
            let receipts =
                reconcile_tick::<()>(&mut settings, utc("2026-10-13T10:00:03Z"), |_| Ok(()))
                    .unwrap()
                    .unwrap();
            assert_eq!(receipts.len(), 1);
            assert_eq!(receipts[0].status, AlarmOccurrenceStatus::Delivered);
        }
    }
    #[test]
    fn alm_12_restart_from_older_cursor_recovers_due_alarm() {
        let mut live = settings();
        let mut saved = live.clone();
        reconcile_tick::<()>(&mut live, utc("2026-10-13T09:59:59Z"), |_| {
            panic!("idle write")
        })
        .unwrap();
        let receipts = reconcile_tick::<()>(&mut saved, utc("2026-10-13T10:01:00Z"), |_| Ok(()))
            .unwrap()
            .unwrap();
        assert_eq!(receipts[0].status, AlarmOccurrenceStatus::Delivered);
        let mut restarted = saved.clone();
        assert!(
            reconcile_tick::<()>(&mut restarted, utc("2026-10-13T10:02:00Z"), |_| panic!(
                "duplicate write"
            ))
            .unwrap()
            .is_none()
        );
        assert_eq!(
            oziclock_app::alarm_time::pending_attention(&restarted.planner).len(),
            1
        );
    }

    #[test]
    fn alm_12_late_restart_records_missed_alarm() {
        let mut settings = settings();
        let receipts = reconcile_tick::<()>(&mut settings, utc("2026-10-13T10:06:00Z"), |_| Ok(()))
            .unwrap()
            .unwrap();
        assert_eq!(receipts[0].status, AlarmOccurrenceStatus::Missed);
    }

    #[test]
    fn alm_16_snooze_removal_requires_successful_save() {
        let mut settings = settings();
        settings.planner.alarms[0].enabled = false;
        settings.planner.alarm_snoozes.push(AlarmSnooze {
            alarm_id: settings.planner.alarms[0].id.clone(),
            due_utc: "2026-10-13T09:01:00Z".into(),
        });
        assert!(reconcile_tick(&mut settings, utc("2026-10-13T09:01:00Z"), |_| Err(())).is_err());
        assert_eq!(settings.planner.alarm_snoozes.len(), 1);
        let receipts = reconcile_tick::<()>(&mut settings, utc("2026-10-13T09:01:01Z"), |_| Ok(()))
            .unwrap()
            .unwrap();
        assert_eq!(receipts.len(), 1);
        assert!(settings.planner.alarm_snoozes.is_empty());
    }

    #[test]
    fn alm_18_history_pruning_is_a_durable_change() {
        let mut settings = settings();
        settings.planner.alarm_receipts.push(AlarmReceipt {
            alarm_id: settings.planner.alarms[0].id.clone(),
            occurrence_utc: "2026-09-01T10:00:00Z".into(),
            recorded_at_utc: "2026-09-01T10:00:00Z".into(),
            status: AlarmOccurrenceStatus::Missed,
            acknowledged_at_utc: None,
        });
        assert!(reconcile_tick(&mut settings, utc("2026-10-13T09:00:01Z"), |_| Err(())).is_err());
        assert_eq!(settings.planner.alarm_receipts.len(), 1);
        assert!(
            reconcile_tick::<()>(&mut settings, utc("2026-10-13T09:00:02Z"), |_| Ok(()))
                .unwrap()
                .unwrap()
                .is_empty()
        );
        assert!(settings.planner.alarm_receipts.is_empty());
    }
}
