use super::{AppSettings, AppWindow, update_clock_tiles};
use chrono::{DateTime, Timelike, Utc};
use slint::{ComponentHandle, Timer, TimerMode};
use std::{cell::RefCell, rc::Rc, time::Duration};

pub(super) fn schedule_clock_refresh(
    timer: Rc<Timer>,
    window: slint::Weak<AppWindow>,
    settings: Rc<RefCell<AppSettings>>,
    explored_time: Rc<RefCell<Option<DateTime<Utc>>>>,
) {
    let now = Utc::now();
    let show_seconds = settings.borrow().show_seconds;
    let milliseconds =
        next_refresh_delay_millis(show_seconds, now.second(), now.timestamp_subsec_millis());
    let timer_for_callback = timer.clone();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(milliseconds.max(1) as u64),
        move || {
            if explored_time.borrow().is_none()
                && let Some(window) = window.upgrade()
                && window.window().is_visible()
            {
                let settings = settings.borrow();
                update_clock_tiles(&window, &settings.clocks_settings, settings.show_seconds);
            }
            schedule_clock_refresh(
                timer_for_callback.clone(),
                window.clone(),
                settings.clone(),
                explored_time.clone(),
            );
        },
    );
}

fn next_refresh_delay_millis(show_seconds: bool, second: u32, millisecond: u32) -> i64 {
    if show_seconds {
        1_000 - i64::from(millisecond)
    } else {
        (60 - i64::from(second)) * 1_000 - i64::from(millisecond)
    }
}

#[cfg(test)]
mod tests {
    use super::next_refresh_delay_millis;

    #[test]
    fn seconds_refresh_aligns_to_the_next_second() {
        assert_eq!(next_refresh_delay_millis(true, 17, 250), 750);
    }

    #[test]
    fn minute_refresh_aligns_to_the_next_minute() {
        assert_eq!(next_refresh_delay_millis(false, 17, 250), 42_750);
    }
}
