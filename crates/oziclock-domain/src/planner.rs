use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct PlannerId(String);

impl PlannerId {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        (!value.trim().is_empty()).then_some(Self(value))
    }
}

impl fmt::Display for PlannerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct AlertRule {
    pub id: PlannerId,
    pub offset_minutes: i32,
    pub play_sound: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum EventTime {
    Timed {
        start_utc: String,
        end_utc: String,
        source_time_zone: String,
    },
    AllDay {
        start_date: String,
        end_date: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Event {
    pub id: PlannerId,
    pub title: String,
    pub time: EventTime,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub link: Option<String>,
    pub color: Option<String>,
    pub alerts: Vec<AlertRule>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum TaskStatus {
    Open,
    Completed,
    Cancelled,
    Archived,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Task {
    pub id: PlannerId,
    pub title: String,
    pub status: TaskStatus,
    pub due_utc: Option<String>,
    pub scheduled_start_utc: Option<String>,
    pub scheduled_end_utc: Option<String>,
    pub color: Option<String>,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub alerts: Vec<AlertRule>,
}

impl Task {
    pub fn complete(&mut self) -> bool {
        if self.status != TaskStatus::Open {
            return false;
        }
        self.status = TaskStatus::Completed;
        true
    }

    pub fn archive(&mut self) -> bool {
        if self.status == TaskStatus::Archived {
            return false;
        }
        self.status = TaskStatus::Archived;
        true
    }

    pub fn reopen(&mut self) -> bool {
        if self.status != TaskStatus::Completed {
            return false;
        }
        self.status = TaskStatus::Open;
        true
    }

    pub fn rename(&mut self, title: String) -> bool {
        let title = title.trim();
        if title.is_empty() {
            return false;
        }
        self.title = title.to_owned();
        true
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum ReminderSchedule {
    Absolute {
        at_utc: String,
        source_time_zone: String,
    },
    ImportantDate {
        month: u8,
        day: u8,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Reminder {
    pub id: PlannerId,
    pub title: String,
    pub schedule: ReminderSchedule,
    pub alerts: Vec<AlertRule>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum AlarmSchedule {
    Once {
        local_date: String,
        local_time: String,
    },
    Weekly {
        local_time: String,
        weekdays: Vec<u8>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Alarm {
    pub id: PlannerId,
    pub title: String,
    pub schedule: AlarmSchedule,
    pub time_zone: String,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum AlarmOccurrenceStatus {
    Delivered,
    Missed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct AlarmReceipt {
    pub alarm_id: PlannerId,
    pub occurrence_utc: String,
    pub status: AlarmOccurrenceStatus,
    pub recorded_at_utc: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub acknowledged_at_utc: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct AlarmSnooze {
    pub alarm_id: PlannerId,
    pub due_utc: String,
}

impl Alarm {
    pub fn has_valid_schedule(&self) -> bool {
        match &self.schedule {
            AlarmSchedule::Once { .. } => true,
            AlarmSchedule::Weekly { weekdays, .. } => {
                !weekdays.is_empty() && weekdays.iter().all(|day| *day <= 6)
            }
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum TimerState {
    Idle,
    Running,
    Paused,
    Finished,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Timer {
    pub id: PlannerId,
    pub title: String,
    pub duration_seconds: u64,
    pub remaining_seconds: u64,
    pub state: TimerState,
    pub repeat: bool,
    pub started_at_utc: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub attention_pending: bool,
}

impl Timer {
    pub fn start(&mut self, started_at_utc: String) -> bool {
        if self.remaining_seconds == 0 {
            self.remaining_seconds = self.duration_seconds;
        }
        if self.remaining_seconds == 0 || self.state == TimerState::Running {
            return false;
        }
        self.state = TimerState::Running;
        self.started_at_utc = Some(started_at_utc);
        self.attention_pending = false;
        true
    }

    pub fn pause(&mut self, remaining_seconds: u64) -> bool {
        if self.state != TimerState::Running {
            return false;
        }
        self.state = TimerState::Paused;
        self.remaining_seconds = remaining_seconds.min(self.duration_seconds);
        self.started_at_utc = None;
        true
    }

    pub fn finish(&mut self) {
        self.state = TimerState::Finished;
        self.remaining_seconds = 0;
        self.started_at_utc = None;
        self.attention_pending = true;
    }

    pub fn dismiss(&mut self) -> bool {
        if self.state != TimerState::Finished {
            if !self.attention_pending {
                return false;
            }
            self.attention_pending = false;
            return true;
        }
        self.state = TimerState::Idle;
        self.attention_pending = false;
        true
    }

    pub fn restart(&mut self, started_at_utc: String) -> bool {
        if self.duration_seconds == 0 {
            return false;
        }
        self.remaining_seconds = self.duration_seconds;
        self.state = TimerState::Running;
        self.started_at_utc = Some(started_at_utc);
        self.attention_pending = false;
        true
    }

    pub fn reset(&mut self) -> bool {
        let changed = self.state != TimerState::Idle
            || self.remaining_seconds != self.duration_seconds
            || self.started_at_utc.is_some()
            || self.attention_pending;
        self.remaining_seconds = self.duration_seconds;
        self.state = TimerState::Idle;
        self.started_at_utc = None;
        self.attention_pending = false;
        changed
    }

    pub fn update(&mut self, title: String, duration_seconds: u64, repeat: bool) -> bool {
        if title.trim().is_empty() || duration_seconds == 0 {
            return false;
        }
        self.title = title;
        self.duration_seconds = duration_seconds;
        self.remaining_seconds = duration_seconds;
        self.repeat = repeat;
        self.state = TimerState::Idle;
        self.started_at_utc = None;
        self.attention_pending = false;
        true
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum StopwatchState {
    Idle,
    Running,
    Paused,
    Interrupted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Stopwatch {
    pub state: StopwatchState,
    pub elapsed_seconds: u64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub elapsed_milliseconds: u16,
    pub laps_seconds: Vec<u64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub laps_milliseconds: Vec<u64>,
}

impl Stopwatch {
    pub fn start(&mut self) -> bool {
        if self.state == StopwatchState::Running {
            return false;
        }
        self.state = StopwatchState::Running;
        true
    }

    pub fn pause(&mut self, elapsed_milliseconds: u64) -> bool {
        if self.state != StopwatchState::Running {
            return false;
        }
        self.state = StopwatchState::Paused;
        self.elapsed_seconds = elapsed_milliseconds / 1_000;
        self.elapsed_milliseconds = (elapsed_milliseconds % 1_000) as u16;
        true
    }

    pub fn interrupt(&mut self) {
        if self.state == StopwatchState::Running {
            self.state = StopwatchState::Interrupted;
        }
    }

    pub fn reset(&mut self) {
        self.state = StopwatchState::Idle;
        self.elapsed_seconds = 0;
        self.elapsed_milliseconds = 0;
        self.laps_seconds.clear();
        self.laps_milliseconds.clear();
    }

    pub fn record_lap(&mut self, elapsed_milliseconds: u64) -> bool {
        if self.state != StopwatchState::Running {
            return false;
        }
        if self.laps_milliseconds.is_empty() && !self.laps_seconds.is_empty() {
            self.laps_milliseconds = self
                .laps_seconds
                .iter()
                .map(|seconds| seconds.saturating_mul(1_000))
                .collect();
            self.laps_seconds.clear();
        }
        self.laps_milliseconds.push(elapsed_milliseconds);
        true
    }

    pub fn undo_lap(&mut self) -> bool {
        if self.laps_milliseconds.pop().is_some() {
            return true;
        }
        self.laps_seconds.pop().is_some()
    }

    pub fn clear_laps(&mut self) -> bool {
        if self.laps_seconds.is_empty() && self.laps_milliseconds.is_empty() {
            return false;
        }
        self.laps_seconds.clear();
        self.laps_milliseconds.clear();
        true
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Planner {
    pub events: Vec<Event>,
    pub tasks: Vec<Task>,
    pub reminders: Vec<Reminder>,
    pub alarms: Vec<Alarm>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub alarm_receipts: Vec<AlarmReceipt>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub alarm_checked_at_utc: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub alarm_snoozes: Vec<AlarmSnooze>,
    pub timers: Vec<Timer>,
    pub stopwatch: Option<Stopwatch>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> PlannerId {
        PlannerId::new(value).unwrap()
    }

    #[test]
    fn timer_transitions_preserve_remaining_duration() {
        let mut timer = Timer {
            id: id("timer"),
            title: "Focus".into(),
            duration_seconds: 60,
            remaining_seconds: 60,
            state: TimerState::Idle,
            repeat: false,
            started_at_utc: None,
            attention_pending: false,
        };
        assert!(timer.start("2026-09-03T10:00:00Z".into()));
        assert!(timer.pause(25));
        assert_eq!(timer.remaining_seconds, 25);
        assert!(timer.start("2026-09-03T10:01:00Z".into()));
        timer.finish();
        assert_eq!(timer.state, TimerState::Finished);
        assert_eq!(timer.remaining_seconds, 0);
        assert!(timer.dismiss());
        assert_eq!(timer.state, TimerState::Idle);
        assert!(timer.start("2026-09-03T10:02:00Z".into()));
        assert_eq!(timer.remaining_seconds, 60);
    }

    #[test]
    fn recurring_alarm_requires_valid_weekdays() {
        let alarm = Alarm {
            id: id("alarm"),
            title: "Wake".into(),
            schedule: AlarmSchedule::Weekly {
                local_time: "07:00".into(),
                weekdays: vec![],
            },
            time_zone: "Europe/Kyiv".into(),
            enabled: true,
        };
        assert!(!alarm.has_valid_schedule());
    }

    #[test]
    fn stopwatch_pause_retains_fractional_seconds() {
        let mut stopwatch = Stopwatch {
            state: StopwatchState::Running,
            elapsed_seconds: 0,
            elapsed_milliseconds: 0,
            laps_seconds: vec![],
            laps_milliseconds: vec![],
        };

        assert!(stopwatch.pause(12_345));
        assert_eq!(stopwatch.elapsed_seconds, 12);
        assert_eq!(stopwatch.elapsed_milliseconds, 345);
        assert!(stopwatch.start());
        assert!(stopwatch.record_lap(12_789));
        assert_eq!(stopwatch.laps_milliseconds, vec![12_789]);
        assert!(stopwatch.undo_lap());
        assert!(stopwatch.laps_milliseconds.is_empty());
        assert!(stopwatch.record_lap(13_000));
        assert!(stopwatch.clear_laps());
        assert!(stopwatch.laps_milliseconds.is_empty());
    }
}
