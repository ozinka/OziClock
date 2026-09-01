use super::{CalendarDayData, CalendarMonthData, CalendarWindow};
use chrono::{Datelike, NaiveDate, Timelike};
use oziclock_app::calendar::{
    CalendarDate, CalendarDay, month_grid, rolling_week_grid, shift_day, shift_month, shift_year,
};
use slint::{ModelRc, VecModel};

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const WEEK_HOUR_HEIGHT: f32 = 34.0;
const WEEK_BUFFER_HOURS: f32 = 48.0;
const WEEK_CONTENT_HOURS: i32 = 144;
const WEEK_VISIBLE_CENTER: f32 = 202.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CalendarView {
    Month,
    Week,
    Year,
}

impl CalendarView {
    pub(super) fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Week,
            2 => Self::Year,
            _ => Self::Month,
        }
    }

    fn index(self) -> i32 {
        match self {
            Self::Month => 0,
            Self::Week => 1,
            Self::Year => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct CalendarState {
    pub(super) view: CalendarView,
    pub(super) cursor: CalendarDate,
    pub(super) week_focus: CalendarDate,
    pub(super) selected: CalendarDate,
    pub(super) monday_first: bool,
    pub(super) light_theme: bool,
}

impl CalendarState {
    pub(super) fn new(today: CalendarDate) -> Self {
        Self {
            view: CalendarView::Month,
            cursor: today,
            week_focus: today,
            selected: today,
            monday_first: true,
            light_theme: true,
        }
    }

    pub(super) fn navigate(&mut self, delta: i32) {
        match self.view {
            CalendarView::Week => {
                let days = i64::from(delta) * 7;
                self.cursor = shift_day(self.cursor, days);
                self.week_focus = shift_day(self.week_focus, days);
            }
            CalendarView::Month => self.cursor = shift_month(self.cursor, delta),
            CalendarView::Year => self.cursor = shift_year(self.cursor, delta),
        }
    }

    pub(super) fn show_week(&mut self, today: CalendarDate) {
        self.view = CalendarView::Week;
        self.cursor = week_start(today);
        self.week_focus = today;
    }

    pub(super) fn select_month(&mut self, month_index: i32) {
        if (0..12).contains(&month_index) {
            self.cursor = CalendarDate::new(self.cursor.year, month_index as u32 + 1, 1)
                .expect("month index is validated");
            self.view = CalendarView::Month;
        }
    }

    pub(super) fn select_date(&mut self, date_id: &str) {
        if let Ok(date) = NaiveDate::parse_from_str(date_id, "%Y-%m-%d") {
            self.selected = date.into();
            self.cursor = date.into();
            self.week_focus = date.into();
        }
    }

    pub(super) fn ensure_week_contains(&mut self, today: CalendarDate) -> bool {
        if self.view != CalendarView::Week || self.cursor == week_start(today) {
            return false;
        }
        self.cursor = week_start(today);
        self.week_focus = today;
        true
    }
}

pub(super) fn initial_week_scroll_y(local_now: chrono::NaiveDateTime) -> f32 {
    let hour = local_now.hour() as f32 + local_now.minute() as f32 / 60.0;
    -(WEEK_BUFFER_HOURS * WEEK_HOUR_HEIGHT + hour * WEEK_HOUR_HEIGHT - WEEK_VISIBLE_CENTER)
}

pub(super) fn normalize_week_scroll(state: &mut CalendarState, y: f32) -> (f32, bool) {
    let day_height = 24.0 * WEEK_HOUR_HEIGHT;
    let start = WEEK_BUFFER_HOURS * WEEK_HOUR_HEIGHT;
    let mut center = -y + WEEK_VISIBLE_CENTER;
    let mut adjusted = y;
    let mut shifted = false;
    while center >= start + day_height {
        state.week_focus = shift_day(state.week_focus, 1);
        state.cursor = week_start(state.week_focus);
        adjusted += day_height;
        center -= day_height;
        shifted = true;
    }
    while center < start {
        state.week_focus = shift_day(state.week_focus, -1);
        state.cursor = week_start(state.week_focus);
        adjusted -= day_height;
        center += day_height;
        shifted = true;
    }
    (adjusted, shifted)
}

pub(super) fn refresh_calendar_window(
    window: &CalendarWindow,
    state: &CalendarState,
    today: CalendarDate,
    local_now: chrono::NaiveDateTime,
) {
    window.set_view(state.view.index());
    window.set_light_theme(state.light_theme);
    window.set_heading(match state.view {
        CalendarView::Year => state.cursor.year.to_string().into(),
        CalendarView::Week => {
            let end = shift_day(state.cursor, 6);
            format!(
                "{} {} – {} {} {}",
                state.cursor.day,
                MONTHS[state.cursor.month as usize - 1],
                end.day,
                MONTHS[end.month as usize - 1],
                end.year
            )
            .into()
        }
        _ => format!(
            "{} {}",
            MONTHS[state.cursor.month as usize - 1],
            state.cursor.year
        )
        .into(),
    });
    window.set_selected_label(format_date(state.selected).into());
    let weekday_labels = if state.view == CalendarView::Week {
        rolling_week_grid(state.cursor)
            .into_iter()
            .map(|day| {
                let date = NaiveDate::from_ymd_opt(day.date.year, day.date.month, day.date.day)
                    .expect("CalendarDate is valid");
                WEEKDAYS[date.weekday().num_days_from_monday() as usize][..3].to_string()
            })
            .collect::<Vec<_>>()
    } else if state.monday_first {
        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
            .into_iter()
            .map(str::to_string)
            .collect()
    } else {
        ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
            .into_iter()
            .map(str::to_string)
            .collect()
    };
    window.set_weekday_labels(ModelRc::new(VecModel::from(
        weekday_labels
            .into_iter()
            .map(Into::into)
            .collect::<Vec<slint::SharedString>>(),
    )));
    window.set_days(day_model(
        month_grid(state.cursor.year, state.cursor.month, state.monday_first),
        state,
        today,
    ));
    let week = rolling_week_grid(state.cursor);
    window.set_week_days(day_model(week.clone(), state, today));
    window.set_months(month_model(state, today));

    window.set_hour_labels(ModelRc::new(VecModel::from(
        (0..=WEEK_CONTENT_HOURS)
            .map(|hour| format!("{:02}:00", hour % 24).into())
            .collect::<Vec<slint::SharedString>>(),
    )));
    let current_week = week.iter().any(|day| day.date == today);
    let current_hour = local_now.hour();
    if current_week {
        let y = (WEEK_BUFFER_HOURS + current_hour as f32) * WEEK_HOUR_HEIGHT
            + local_now.minute() as f32 / 60.0 * WEEK_HOUR_HEIGHT;
        window.set_current_time_y(y);
        window.set_current_time_label(
            format!("{:02}:{:02}", current_hour, local_now.minute()).into(),
        );
    } else {
        window.set_current_time_y(-1.0);
        window.set_current_time_label("".into());
    }
}

fn month_model(state: &CalendarState, today: CalendarDate) -> ModelRc<CalendarMonthData> {
    ModelRc::new(VecModel::from(
        (1..=12)
            .map(|month| CalendarMonthData {
                name: MONTHS[month as usize - 1].into(),
                current: today.year == state.cursor.year && today.month == month,
                days: day_model(
                    month_grid(state.cursor.year, month, state.monday_first),
                    state,
                    today,
                ),
            })
            .collect::<Vec<_>>(),
    ))
}

fn day_model(
    days: Vec<CalendarDay>,
    state: &CalendarState,
    today: CalendarDate,
) -> ModelRc<CalendarDayData> {
    ModelRc::new(VecModel::from(
        days.into_iter()
            .map(|day| CalendarDayData {
                text: day.date.day.to_string().into(),
                date_id: format!(
                    "{:04}-{:02}-{:02}",
                    day.date.year, day.date.month, day.date.day
                )
                .into(),
                weekend: day.weekend,
                muted: day.outside_month,
                selected: day.date == state.selected,
                today: day.date == today,
                focused: day.date == state.week_focus,
            })
            .collect::<Vec<_>>(),
    ))
}

fn week_start(date: CalendarDate) -> CalendarDate {
    let value =
        NaiveDate::from_ymd_opt(date.year, date.month, date.day).expect("CalendarDate is valid");
    shift_day(date, -i64::from(value.weekday().num_days_from_monday()))
}

fn format_date(date: CalendarDate) -> String {
    let value =
        NaiveDate::from_ymd_opt(date.year, date.month, date.day).expect("CalendarDate is valid");
    let weekday = WEEKDAYS[value.weekday().num_days_from_monday() as usize];
    format!(
        "{weekday}, {} {} {}",
        date.day,
        MONTHS[date.month as usize - 1],
        date.year
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_time_is_centered_in_the_twelve_hour_viewport() {
        let now = NaiveDate::from_ymd_opt(2026, 8, 30)
            .unwrap()
            .and_hms_opt(18, 30, 0)
            .unwrap();
        let scroll = initial_week_scroll_y(now);
        let current_y = (WEEK_BUFFER_HOURS + 18.5) * WEEK_HOUR_HEIGHT;
        assert_eq!(current_y + scroll, WEEK_VISIBLE_CENTER);
    }

    #[test]
    fn crossing_midnight_shifts_the_range_by_one_day_without_a_jump() {
        let mut state = CalendarState::new(CalendarDate::new(2026, 8, 30).unwrap());
        state.view = CalendarView::Week;
        let midnight_scroll = -(WEEK_BUFFER_HOURS * WEEK_HOUR_HEIGHT + 24.0 * WEEK_HOUR_HEIGHT);
        let (adjusted, shifted) = normalize_week_scroll(&mut state, midnight_scroll);

        assert!(shifted);
        assert_eq!(state.cursor, CalendarDate::new(2026, 8, 31).unwrap());
        assert_eq!(adjusted, midnight_scroll + 24.0 * WEEK_HOUR_HEIGHT);
    }

    #[test]
    fn week_view_returns_to_today_when_its_range_has_expired() {
        let mut state = CalendarState::new(CalendarDate::new(2026, 8, 30).unwrap());
        state.view = CalendarView::Week;
        let today = CalendarDate::new(2026, 9, 6).unwrap();

        assert!(state.ensure_week_contains(today));
        assert_eq!(state.cursor, CalendarDate::new(2026, 8, 31).unwrap());
        assert_eq!(state.week_focus, today);
    }
}
