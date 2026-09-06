//! Countdown reconstruction and reconciliation, independent of UI refreshes.

use chrono::{DateTime, Utc};
use oziclock_domain::{Planner, PlannerId, Timer, TimerState};

pub fn remaining_seconds(timer: &Timer, now: DateTime<Utc>) -> u64 {
    if timer.state != TimerState::Running {
        return timer.remaining_seconds;
    }
    let elapsed_seconds = timer
        .started_at_utc
        .as_deref()
        .and_then(|started| DateTime::parse_from_rfc3339(started).ok())
        .map(|started| {
            now.signed_duration_since(started.with_timezone(&Utc))
                .num_seconds()
                .max(0) as u64
        })
        .unwrap_or_default();
    timer.remaining_seconds.saturating_sub(elapsed_seconds)
}

pub fn reconcile(planner: &mut Planner, now: DateTime<Utc>) -> Vec<PlannerId> {
    let due = planner
        .timers
        .iter()
        .filter(|timer| timer.state == TimerState::Running && remaining_seconds(timer, now) == 0)
        .map(|timer| timer.id.clone())
        .collect::<Vec<_>>();
    let mut newly_pending = Vec::new();
    for id in &due {
        if let Some(timer) = planner.timers.iter_mut().find(|timer| &timer.id == id) {
            let was_pending = timer.attention_pending;
            timer.finish();
            if !was_pending {
                timer.attention_triggered_at_utc = Some(now.to_rfc3339());
            }
            let should_repeat = timer.repeat
                && match timer.repeats_remaining {
                    Some(remaining) if remaining > 0 => {
                        timer.repeats_remaining = Some(remaining - 1);
                        true
                    }
                    Some(_) => false,
                    None => true,
                };
            if should_repeat {
                timer.state = TimerState::Running;
                timer.remaining_seconds = timer.duration_seconds;
                timer.started_at_utc = Some(now.to_rfc3339());
            }
            if !was_pending {
                newly_pending.push(id.clone());
            }
        }
    }
    newly_pending
}

pub fn pending_attention(planner: &Planner) -> Vec<PlannerId> {
    planner
        .timers
        .iter()
        .filter(|timer| timer.attention_pending || timer.state == TimerState::Finished)
        .map(|timer| timer.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(value: &str) -> DateTime<Utc> {
        value.parse().unwrap()
    }

    fn running_timer(started_at_utc: &str) -> Timer {
        Timer {
            id: PlannerId::new("timer").unwrap(),
            title: "Focus".into(),
            duration_seconds: 300,
            remaining_seconds: 300,
            state: TimerState::Running,
            repeat: false,
            repeat_count: Some(0),
            repeats_remaining: Some(0),
            started_at_utc: Some(started_at_utc.into()),
            attention_pending: false,
            attention_triggered_at_utc: None,
        }
    }

    #[test]
    fn reconstructs_remaining_time_after_restart() {
        let timer = running_timer("2026-09-05T10:00:00Z");
        assert_eq!(remaining_seconds(&timer, utc("2026-09-05T10:02:03Z")), 177);
        assert_eq!(remaining_seconds(&timer, utc("2026-09-05T09:59:00Z")), 300);
    }

    #[test]
    fn invalid_start_does_not_consume_the_timer() {
        let timer = running_timer("invalid");
        assert_eq!(remaining_seconds(&timer, utc("2026-09-05T10:10:00Z")), 300);
    }

    #[test]
    fn reconciliation_finishes_each_expired_timer_once() {
        let mut planner = Planner {
            timers: vec![running_timer("2026-09-05T10:00:00Z")],
            ..Planner::default()
        };
        let now = utc("2026-09-05T10:05:00Z");
        assert_eq!(reconcile(&mut planner, now).len(), 1);
        assert_eq!(planner.timers[0].state, TimerState::Finished);
        assert_eq!(planner.timers[0].remaining_seconds, 0);
        assert!(reconcile(&mut planner, now).is_empty());
        assert_eq!(
            pending_attention(&planner),
            vec![PlannerId::new("timer").unwrap()]
        );
    }

    #[test]
    fn repeating_timer_restarts_and_keeps_attention_pending() {
        let mut timer = running_timer("2026-09-05T10:00:00Z");
        timer.repeat = true;
        timer.repeat_count = None;
        timer.repeats_remaining = None;
        let mut planner = Planner {
            timers: vec![timer],
            ..Planner::default()
        };
        let now = utc("2026-09-05T10:05:00Z");

        assert_eq!(reconcile(&mut planner, now).len(), 1);
        assert_eq!(planner.timers[0].state, TimerState::Running);
        assert_eq!(planner.timers[0].remaining_seconds, 300);
        assert_eq!(
            planner.timers[0].started_at_utc.as_deref(),
            Some("2026-09-05T10:05:00+00:00")
        );
        assert!(planner.timers[0].attention_pending);
        assert_eq!(pending_attention(&planner).len(), 1);
    }

    #[test]
    fn finite_repeat_count_means_additional_runs_and_survives_pending_attention() {
        let mut timer = running_timer("2026-09-05T10:00:00Z");
        timer.repeat = true;
        timer.repeat_count = Some(2);
        timer.repeats_remaining = Some(2);
        let mut planner = Planner {
            timers: vec![timer],
            ..Planner::default()
        };

        assert_eq!(
            reconcile(&mut planner, utc("2026-09-05T10:05:00Z")).len(),
            1
        );
        assert_eq!(planner.timers[0].state, TimerState::Running);
        assert_eq!(planner.timers[0].repeats_remaining, Some(1));
        let first_trigger = planner.timers[0].attention_triggered_at_utc.clone();

        assert!(reconcile(&mut planner, utc("2026-09-05T10:10:00Z")).is_empty());
        assert_eq!(planner.timers[0].state, TimerState::Running);
        assert_eq!(planner.timers[0].repeats_remaining, Some(0));
        assert_eq!(planner.timers[0].attention_triggered_at_utc, first_trigger);

        assert!(reconcile(&mut planner, utc("2026-09-05T10:15:00Z")).is_empty());
        assert_eq!(planner.timers[0].state, TimerState::Finished);
        assert_eq!(planner.timers[0].repeats_remaining, Some(0));
    }
}
