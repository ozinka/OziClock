use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use chrono_tz::Tz;
use oziclock_storage::{
    AlarmOccurrenceStatus, AlarmReceipt, AlarmSchedule, AppSettings, EventRecurrence, EventTime,
    PlannerId, PlannerTimer, ReminderRecurrence, ReminderSchedule, Stopwatch, TaskStatus,
    TimerState,
};
use slint::ModelRc;

use super::{
    PlanAllDayEventData, PlanEventMarkerData, PlanReminderMarkerData, PlanTaskMarkerData,
    PlannerAlarmData, PlannerReminderData, PlannerTaskData, PlannerTimerData, StopwatchLapData,
};

pub(super) fn open_task_count(settings: &AppSettings) -> i32 {
    settings
        .planner
        .tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Open)
        .count() as i32
}

pub(super) fn completed_task_count(settings: &AppSettings) -> i32 {
    settings
        .planner
        .tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Completed)
        .count() as i32
}

pub(super) fn planner_task_rows(settings: &AppSettings) -> Vec<PlannerTaskData> {
    settings
        .planner
        .tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Open)
        .map(|task| PlannerTaskData {
            id: task.id.to_string().into(),
            title: task.title.clone().into(),
        })
        .collect()
}

pub(super) fn planner_completed_task_rows(settings: &AppSettings) -> Vec<PlannerTaskData> {
    settings
        .planner
        .tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Completed)
        .map(|task| PlannerTaskData {
            id: task.id.to_string().into(),
            title: task.title.clone().into(),
        })
        .collect()
}

pub(super) fn planner_alarm_rows(settings: &AppSettings) -> Vec<PlannerAlarmData> {
    settings
        .planner
        .alarms
        .iter()
        .map(|alarm| {
            let (schedule, time, weekly, weekdays) = match &alarm.schedule {
                AlarmSchedule::Once {
                    local_date,
                    local_time,
                } => (
                    local_date.clone(),
                    local_time.clone(),
                    false,
                    vec![false; 7],
                ),
                AlarmSchedule::Weekly {
                    local_time,
                    weekdays,
                } => (
                    alarm_schedule_label(&alarm.schedule),
                    local_time.clone(),
                    true,
                    (0..7).map(|day| weekdays.contains(&(day as u8))).collect(),
                ),
            };
            PlannerAlarmData {
                id: alarm.id.to_string().into(),
                title: alarm.title.clone().into(),
                schedule: schedule.into(),
                time: time.into(),
                status: alarm_status(&settings.planner.alarm_receipts, &alarm.id).into(),
                weekly,
                weekdays: ModelRc::from(weekdays.as_slice()),
                enabled: alarm.enabled,
            }
        })
        .collect()
}

fn alarm_status(receipts: &[AlarmReceipt], alarm_id: &PlannerId) -> &'static str {
    receipts
        .iter()
        .filter(|receipt| &receipt.alarm_id == alarm_id)
        .max_by(|left, right| left.recorded_at_utc.cmp(&right.recorded_at_utc))
        .map_or("", |receipt| match receipt.status {
            AlarmOccurrenceStatus::Delivered => "Delivered",
            AlarmOccurrenceStatus::Missed => "Missed",
        })
}

fn alarm_schedule_label(schedule: &AlarmSchedule) -> String {
    match schedule {
        AlarmSchedule::Once {
            local_date,
            local_time,
        } => format!("Once · {local_date} at {local_time}"),
        AlarmSchedule::Weekly {
            local_time,
            weekdays,
        } => {
            let days = weekdays
                .iter()
                .filter_map(|day| {
                    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].get(*day as usize)
                })
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
            format!("Weekly · {days} at {local_time}")
        }
    }
}

pub(super) fn planner_timer_rows(settings: &AppSettings) -> Vec<PlannerTimerData> {
    let now = Utc::now();
    settings
        .planner
        .timers
        .iter()
        .map(|timer| {
            let remaining_seconds = oziclock_app::timer_time::remaining_seconds(timer, now);
            PlannerTimerData {
                id: timer.id.to_string().into(),
                title: timer_display_title(timer).into(),
                remaining: format_duration(remaining_seconds).into(),
                running: timer.state == TimerState::Running,
                finished: timer.state == TimerState::Finished,
                repeat: timer.repeat,
                repeat_label: if timer.repeat {
                    timer
                        .repeat_count
                        .map(|count| format!("Repeats ×{count}"))
                        .unwrap_or_else(|| "Repeats ∞".to_owned())
                } else {
                    String::new()
                }
                .into(),
            }
        })
        .collect()
}

pub(super) fn planner_reminder_rows(settings: &AppSettings) -> Vec<PlannerReminderData> {
    settings
        .planner
        .reminders
        .iter()
        .map(|reminder| {
            let due = if reminder.attention_pending {
                "Delivered".to_owned()
            } else {
                format_reminder_schedule(&reminder.schedule, reminder.enabled)
            };
            PlannerReminderData {
                id: reminder.id.to_string().into(),
                title: reminder.title.clone().into(),
                due: due.into(),
                enabled: reminder.enabled,
                delivered: reminder.attention_pending,
            }
        })
        .collect()
}

fn format_reminder_schedule(schedule: &ReminderSchedule, enabled: bool) -> String {
    let (at_utc, source_time_zone, recurrence) = match schedule {
        ReminderSchedule::Absolute {
            at_utc,
            source_time_zone,
        } => (at_utc, source_time_zone, "Once".to_owned()),
        ReminderSchedule::Recurring {
            recurrence,
            next_at_utc,
            source_time_zone,
            ..
        } => {
            let label = match recurrence {
                ReminderRecurrence::Daily => "Daily".to_owned(),
                ReminderRecurrence::Weekly { weekdays } => format!(
                    "Weekly ({})",
                    weekdays
                        .iter()
                        .map(|day| ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                            [usize::from(*day)])
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                ReminderRecurrence::Monthly { .. } => "Monthly".to_owned(),
                ReminderRecurrence::Yearly { .. } => "Yearly".to_owned(),
            };
            (next_at_utc, source_time_zone, label)
        }
        ReminderSchedule::ImportantDate { .. } => return "Important date".to_owned(),
    };
    let Some(due) = DateTime::parse_from_rfc3339(at_utc).ok() else {
        return "Invalid date".to_owned();
    };
    let Some(zone) = source_time_zone.parse::<Tz>().ok() else {
        return "Invalid time zone".to_owned();
    };
    let local = due.with_timezone(&zone);
    let prefix = if enabled { "" } else { "Paused · " };
    format!(
        "{prefix}{recurrence} · {} · {} · {source_time_zone}",
        local.format("%Y-%m-%d"),
        local.format("%H:%M")
    )
}

pub(super) fn plan_counts_for_date(settings: &AppSettings, date: NaiveDate) -> (i32, i32) {
    let week = date - chrono::Duration::days(date.weekday().num_days_from_monday().into());
    let day = date.signed_duration_since(week).num_days() as i32;
    let (timed, all_day) = plan_event_rows(settings, week);
    let events = timed.iter().filter(|item| item.day_index == day).count()
        + all_day.iter().filter(|item| item.day_index == day).count();
    let tasks = plan_task_rows(settings, week)
        .iter()
        .filter(|item| item.day_index == day)
        .count();
    (events as i32, tasks as i32)
}

pub(super) fn plan_reminder_rows(
    settings: &AppSettings,
    start: NaiveDate,
) -> Vec<PlanReminderMarkerData> {
    let zone = super::main_time_zone(settings)
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC);
    let rows = settings
        .planner
        .reminders
        .iter()
        .filter(|reminder| reminder.enabled)
        .filter_map(|reminder| {
            let local =
                oziclock_app::reminder_time::due_utc(&reminder.schedule)?.with_timezone(&zone);
            let day_index = local.date_naive().signed_duration_since(start).num_days();
            let minutes = i32::try_from(local.time().num_seconds_from_midnight() / 60).ok()?;
            (0..7).contains(&day_index).then_some(())?;
            Some(PlanReminderMarkerData {
                id: reminder.id.to_string().into(),
                title: reminder.title.clone().into(),
                date: local.format("%Y-%m-%d").to_string().into(),
                time: local.format("%H:%M").to_string().into(),
                day_index: day_index as i32,
                minute_offset: minutes,
                lane_index: 0,
                lane_count: 1,
            })
        })
        .collect();
    arrange_plan_reminder_lanes(rows)
}

pub(super) fn plan_task_rows(settings: &AppSettings, start: NaiveDate) -> Vec<PlanTaskMarkerData> {
    let zone = super::main_time_zone(settings)
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC);
    settings
        .planner
        .tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Open)
        .filter_map(|task| {
            let due = DateTime::parse_from_rfc3339(task.due_utc.as_deref()?)
                .ok()?
                .with_timezone(&zone);
            let day_index = due.date_naive().signed_duration_since(start).num_days();
            (0..7).contains(&day_index).then_some(PlanTaskMarkerData {
                id: task.id.to_string().into(),
                title: task.title.clone().into(),
                date: due.format("%Y-%m-%d").to_string().into(),
                time: due.format("%H:%M").to_string().into(),
                day_index: day_index as i32,
                minute_offset: i32::try_from(due.time().num_seconds_from_midnight() / 60).ok()?,
            })
        })
        .collect()
}

fn arrange_plan_reminder_lanes(
    mut rows: Vec<PlanReminderMarkerData>,
) -> Vec<PlanReminderMarkerData> {
    const VISUAL_DURATION_MINUTES: i32 = 30;
    rows.sort_by(|left, right| {
        left.day_index
            .cmp(&right.day_index)
            .then(left.minute_offset.cmp(&right.minute_offset))
            .then(left.id.as_str().cmp(right.id.as_str()))
    });
    let mut cluster_start = 0;
    while cluster_start < rows.len() {
        let day = rows[cluster_start].day_index;
        let mut cluster_end = rows[cluster_start].minute_offset + VISUAL_DURATION_MINUTES;
        let mut cluster_limit = cluster_start + 1;
        while cluster_limit < rows.len()
            && rows[cluster_limit].day_index == day
            && rows[cluster_limit].minute_offset < cluster_end
        {
            cluster_end =
                cluster_end.max(rows[cluster_limit].minute_offset + VISUAL_DURATION_MINUTES);
            cluster_limit += 1;
        }

        let mut lane_ends = Vec::<i32>::new();
        for row in &mut rows[cluster_start..cluster_limit] {
            let lane = lane_ends
                .iter()
                .position(|end| *end <= row.minute_offset)
                .unwrap_or(lane_ends.len());
            if lane == lane_ends.len() {
                lane_ends.push(0);
            }
            lane_ends[lane] = row.minute_offset + VISUAL_DURATION_MINUTES;
            row.lane_index = lane as i32;
        }
        let lane_count = lane_ends.len().max(1) as i32;
        for row in &mut rows[cluster_start..cluster_limit] {
            row.lane_count = lane_count;
        }
        cluster_start = cluster_limit;
    }
    rows
}

pub(super) fn plan_event_rows(
    settings: &AppSettings,
    week_start: NaiveDate,
) -> (Vec<PlanEventMarkerData>, Vec<PlanAllDayEventData>) {
    let zone = super::main_time_zone(settings)
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC);
    let week_end = week_start + chrono::Duration::days(6);
    let mut timed = Vec::new();
    let mut all_day = Vec::new();

    for event in &settings.planner.events {
        match &event.time {
            EventTime::Timed {
                start_utc,
                end_utc,
                source_time_zone,
            } => {
                let (Ok(start), Ok(end)) = (
                    DateTime::parse_from_rfc3339(start_utc),
                    DateTime::parse_from_rfc3339(end_utc),
                ) else {
                    continue;
                };
                let source_zone = source_time_zone.parse::<Tz>().unwrap_or(chrono_tz::UTC);
                let source_start = start.with_timezone(&source_zone);
                let duration = end.with_timezone(&Utc) - start.with_timezone(&Utc);
                for occurrence_date in event_occurrence_dates(
                    source_start.date_naive(),
                    event.recurrence,
                    week_start - chrono::Duration::days(2),
                    week_end + chrono::Duration::days(2),
                ) {
                    let Some(occurrence_start) = oziclock_app::reminder_time::resolve_local(
                        &occurrence_date.format("%Y-%m-%d").to_string(),
                        &source_start.format("%H:%M").to_string(),
                        source_time_zone,
                    ) else {
                        continue;
                    };
                    let start = occurrence_start.with_timezone(&zone);
                    let end = (occurrence_start + duration).with_timezone(&zone);
                    let date = start.date_naive();
                    let day_index = date.signed_duration_since(week_start).num_days();
                    if !(0..7).contains(&day_index) {
                        continue;
                    }
                    let start_minute = i32::try_from(start.time().num_seconds_from_midnight() / 60)
                        .unwrap_or_default();
                    let end_minute = if end.date_naive() == date {
                        i32::try_from(end.time().num_seconds_from_midnight() / 60).unwrap_or(1_440)
                    } else {
                        1_440
                    };
                    let visible_end = end_minute.min(1_440);
                    if visible_end <= start_minute {
                        continue;
                    }
                    timed.push(PlanEventMarkerData {
                        id: event.id.to_string().into(),
                        title: event.title.clone().into(),
                        date: date.format("%Y-%m-%d").to_string().into(),
                        time: start.format("%H:%M").to_string().into(),
                        day_index: day_index as i32,
                        minute_offset: start_minute,
                        duration_minutes: visible_end - start_minute,
                        lane_index: 0,
                        lane_count: 1,
                    });
                }
            }
            EventTime::AllDay {
                start_date,
                end_date,
            } => {
                let (Ok(start), Ok(end)) = (
                    NaiveDate::parse_from_str(start_date, "%Y-%m-%d"),
                    NaiveDate::parse_from_str(end_date, "%Y-%m-%d"),
                ) else {
                    continue;
                };
                let duration_days = end.signed_duration_since(start).num_days();
                for occurrence_start in event_occurrence_dates(
                    start,
                    event.recurrence,
                    week_start - chrono::Duration::days(duration_days),
                    week_end,
                ) {
                    let occurrence_end = occurrence_start + chrono::Duration::days(duration_days);
                    let first = occurrence_start.max(week_start);
                    let last = occurrence_end.min(week_end);
                    for offset in 0..=last.signed_duration_since(first).num_days() {
                        let date = first + chrono::Duration::days(offset);
                        all_day.push(PlanAllDayEventData {
                            id: event.id.to_string().into(),
                            title: event.title.clone().into(),
                            date: date.format("%Y-%m-%d").to_string().into(),
                            day_index: date.signed_duration_since(week_start).num_days() as i32,
                        });
                    }
                }
            }
        }
    }
    arrange_plan_event_lanes(&mut timed);
    all_day.sort_by_key(|event| event.day_index);
    (timed, all_day)
}

fn event_occurrence_dates(
    origin: NaiveDate,
    recurrence: EventRecurrence,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> Vec<NaiveDate> {
    if recurrence == EventRecurrence::None {
        return (origin >= range_start && origin <= range_end)
            .then_some(origin)
            .into_iter()
            .collect();
    }
    let first = range_start.max(origin);
    (0..=range_end.signed_duration_since(first).num_days())
        .map(|offset| first + chrono::Duration::days(offset))
        .filter(|date| match recurrence {
            EventRecurrence::None => false,
            EventRecurrence::Daily => true,
            EventRecurrence::Weekly => date.weekday() == origin.weekday(),
            EventRecurrence::Monthly => date.day() == origin.day(),
            EventRecurrence::Yearly => date.month() == origin.month() && date.day() == origin.day(),
        })
        .collect()
}

fn arrange_plan_event_lanes(rows: &mut [PlanEventMarkerData]) {
    rows.sort_by(|left, right| {
        left.day_index
            .cmp(&right.day_index)
            .then(left.minute_offset.cmp(&right.minute_offset))
            .then(left.id.as_str().cmp(right.id.as_str()))
    });
    let mut cluster_start = 0;
    while cluster_start < rows.len() {
        let day = rows[cluster_start].day_index;
        let mut cluster_end =
            rows[cluster_start].minute_offset + rows[cluster_start].duration_minutes;
        let mut cluster_limit = cluster_start + 1;
        while cluster_limit < rows.len()
            && rows[cluster_limit].day_index == day
            && rows[cluster_limit].minute_offset < cluster_end
        {
            cluster_end = cluster_end
                .max(rows[cluster_limit].minute_offset + rows[cluster_limit].duration_minutes);
            cluster_limit += 1;
        }
        let mut lane_ends = Vec::<i32>::new();
        for row in &mut rows[cluster_start..cluster_limit] {
            let lane = lane_ends
                .iter()
                .position(|end| *end <= row.minute_offset)
                .unwrap_or(lane_ends.len());
            if lane == lane_ends.len() {
                lane_ends.push(0);
            }
            lane_ends[lane] = row.minute_offset + row.duration_minutes;
            row.lane_index = lane as i32;
        }
        let lane_count = lane_ends.len().max(1) as i32;
        for row in &mut rows[cluster_start..cluster_limit] {
            row.lane_count = lane_count;
        }
        cluster_start = cluster_limit;
    }
}

fn format_duration(total_seconds: u64) -> String {
    format!("{:02}:{:02}", total_seconds / 60, total_seconds % 60)
}

fn format_timer_duration(total_seconds: u64) -> String {
    let days = total_seconds / 86_400;
    let hours = total_seconds % 86_400 / 3_600;
    let minutes = total_seconds % 3_600 / 60;
    let seconds = total_seconds % 60;
    if days > 0 {
        format!("{days}d {hours:02}:{minutes:02}:{seconds:02}")
    } else if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

pub(super) fn timer_display_title(timer: &PlannerTimer) -> String {
    format!(
        "{} — {}",
        timer.title,
        format_timer_duration(timer.duration_seconds)
    )
}

pub(super) fn format_stopwatch(milliseconds: u128) -> (String, String) {
    let total_seconds = milliseconds / 1_000;
    let days = total_seconds / 86_400;
    let hours = total_seconds % 86_400 / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;
    (
        if days > 0 {
            format!("{days}d {hours:02}:{minutes:02}:{seconds:02}")
        } else {
            format!("{hours:02}:{minutes:02}:{seconds:02}")
        },
        format!(".{:02}", (milliseconds % 1_000) / 10),
    )
}

fn format_lap(milliseconds: u64) -> String {
    let total_seconds = milliseconds / 1_000;
    let days = total_seconds / 86_400;
    let hours = total_seconds % 86_400 / 3_600;
    let minutes = total_seconds % 3_600 / 60;
    let seconds = total_seconds % 60;
    let centiseconds = milliseconds % 1_000 / 10;
    if days > 0 {
        format!("{days}d {hours:02}:{minutes:02}:{seconds:02}.{centiseconds:02}")
    } else {
        format!("{hours:02}:{minutes:02}:{seconds:02}.{centiseconds:02}")
    }
}

pub(super) fn stopwatch_lap_rows(settings: &AppSettings) -> Vec<StopwatchLapData> {
    let Some(stopwatch) = settings.planner.stopwatch.as_ref() else {
        return Vec::new();
    };
    stopwatch_lap_rows_from(stopwatch)
}

fn stopwatch_lap_rows_from(stopwatch: &Stopwatch) -> Vec<StopwatchLapData> {
    let totals = if stopwatch.laps_milliseconds.is_empty() {
        stopwatch
            .laps_seconds
            .iter()
            .map(|seconds| seconds.saturating_mul(1_000))
            .collect::<Vec<_>>()
    } else {
        stopwatch.laps_milliseconds.clone()
    };
    let splits = totals
        .iter()
        .scan(0_u64, |previous, total| {
            let split = total.saturating_sub(*previous);
            *previous = *total;
            Some(split)
        })
        .collect::<Vec<_>>();
    let fastest = (splits.len() > 1)
        .then(|| splits.iter().copied().min())
        .flatten();
    let slowest = (splits.len() > 1)
        .then(|| splits.iter().copied().max())
        .flatten();
    let distinguish_extremes = fastest != slowest;
    totals
        .iter()
        .zip(splits.iter())
        .enumerate()
        .rev()
        .map(|(index, (total, split))| StopwatchLapData {
            number: (index + 1) as i32,
            total: format_lap(*total).into(),
            split: format_lap(*split).into(),
            fastest: distinguish_extremes && Some(*split) == fastest,
            slowest: distinguish_extremes && Some(*split) == slowest,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_marker(id: &str, day: i32, minute: i32) -> PlanReminderMarkerData {
        PlanReminderMarkerData {
            id: id.into(),
            title: id.into(),
            date: "2026-09-07".into(),
            time: "10:00".into(),
            day_index: day,
            minute_offset: minute,
            lane_index: 0,
            lane_count: 1,
        }
    }

    fn plan_event(id: &str, day: i32, minute: i32, duration: i32) -> PlanEventMarkerData {
        PlanEventMarkerData {
            id: id.into(),
            title: id.into(),
            date: "2026-09-07".into(),
            time: "10:00".into(),
            day_index: day,
            minute_offset: minute,
            duration_minutes: duration,
            lane_index: 0,
            lane_count: 1,
        }
    }

    #[test]
    fn evt_07_overlapping_plan_events_share_lanes_until_the_cluster_ends() {
        let mut rows = vec![
            plan_event("long", 0, 60, 90),
            plan_event("short", 0, 90, 30),
            plan_event("later", 0, 180, 30),
        ];
        arrange_plan_event_lanes(&mut rows);
        assert_eq!((rows[0].lane_index, rows[0].lane_count), (0, 2));
        assert_eq!((rows[1].lane_index, rows[1].lane_count), (1, 2));
        assert_eq!((rows[2].lane_index, rows[2].lane_count), (0, 1));
    }

    #[test]
    fn evt_08_recurrence_expands_only_matching_dates_after_the_origin() {
        let origin = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let range_end = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
        assert_eq!(
            event_occurrence_dates(origin, EventRecurrence::Weekly, origin, range_end),
            vec![origin, NaiveDate::from_ymd_opt(2026, 9, 14).unwrap()]
        );
        assert_eq!(
            event_occurrence_dates(
                origin,
                EventRecurrence::Daily,
                origin - chrono::Duration::days(3),
                origin + chrono::Duration::days(2),
            ),
            vec![
                origin,
                NaiveDate::from_ymd_opt(2026, 9, 8).unwrap(),
                NaiveDate::from_ymd_opt(2026, 9, 9).unwrap(),
            ]
        );
    }

    #[test]
    fn pln_06_overlapping_plan_reminders_share_deterministic_lanes() {
        let rows = arrange_plan_reminder_lanes(vec![
            plan_marker("later", 1, 80),
            plan_marker("first", 0, 60),
            plan_marker("overlap", 0, 75),
            plan_marker("separate", 0, 120),
        ]);

        assert_eq!(rows[0].id.as_str(), "first");
        assert_eq!((rows[0].lane_index, rows[0].lane_count), (0, 2));
        assert_eq!((rows[1].lane_index, rows[1].lane_count), (1, 2));
        assert_eq!((rows[2].lane_index, rows[2].lane_count), (0, 1));
        assert_eq!((rows[3].lane_index, rows[3].lane_count), (0, 1));
    }

    #[test]
    fn alm_17_alarm_status_uses_latest_receipt_for_that_alarm() {
        let alarm_id = PlannerId::new("alarm").unwrap();
        let other_id = PlannerId::new("other").unwrap();
        let receipt = |id: PlannerId, recorded: &str, status: AlarmOccurrenceStatus| AlarmReceipt {
            alarm_id: id,
            occurrence_utc: recorded.into(),
            status,
            recorded_at_utc: recorded.into(),
            acknowledged_at_utc: None,
        };
        let receipts = vec![
            receipt(
                alarm_id.clone(),
                "2026-09-05T09:00:00+00:00",
                AlarmOccurrenceStatus::Missed,
            ),
            receipt(
                other_id,
                "2026-09-05T11:00:00+00:00",
                AlarmOccurrenceStatus::Missed,
            ),
            receipt(
                alarm_id.clone(),
                "2026-09-05T10:00:00+00:00",
                AlarmOccurrenceStatus::Delivered,
            ),
        ];
        assert_eq!(alarm_status(&receipts, &alarm_id), "Delivered");
        assert_eq!(
            alarm_status(&receipts, &PlannerId::new("none").unwrap()),
            ""
        );
    }

    #[test]
    fn tmr_06_duration_label_keeps_the_original_scale_visible() {
        assert_eq!(format_timer_duration(300), "05:00");
        assert_eq!(format_timer_duration(3_661), "01:01:01");
        assert_eq!(format_timer_duration(90_061), "1d 01:01:01");
    }

    #[test]
    fn sw_02_sw_04_sw_05_laps_keep_centiseconds_splits_and_extremes() {
        let stopwatch = Stopwatch {
            state: oziclock_storage::StopwatchState::Paused,
            elapsed_seconds: 10,
            elapsed_milliseconds: 0,
            laps_seconds: vec![],
            laps_milliseconds: vec![1_250, 3_750, 5_000],
        };
        let rows = stopwatch_lap_rows_from(&stopwatch);
        assert_eq!(rows[0].number, 3);
        assert_eq!(rows[0].split.as_str(), "00:00:01.25");
        assert!(rows[0].fastest);
        assert_eq!(rows[1].split.as_str(), "00:00:02.50");
        assert!(rows[1].slowest);
        assert_eq!(rows[2].total.as_str(), "00:00:01.25");
    }

    #[test]
    fn sw_10_stopwatch_format_supports_sessions_longer_than_a_day() {
        assert_eq!(format_stopwatch(90_061_230).0, "1d 01:01:01");
        assert_eq!(format_lap(90_061_230), "1d 01:01:01.23");
    }
}
