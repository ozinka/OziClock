//! Framework-free domain types and rules for OziClock.

mod clock;
mod planner;

pub use clock::{Clock, ClockCollection};
pub use planner::{
    Alarm, AlarmOccurrenceStatus, AlarmReceipt, AlarmSchedule, AlarmSnooze, AlertRule, Event,
    EventReceipt, EventRecurrence, EventTime, Planner, PlannerId, Reminder, ReminderRecurrence,
    ReminderSchedule, Stopwatch, StopwatchState, Task, TaskStatus, Timer, TimerState,
};

/// Product name shared by all front ends.
pub const PRODUCT_NAME: &str = "OziClock";
