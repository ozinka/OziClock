use chrono::{Datelike, Duration, NaiveDate, Weekday};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl CalendarDate {
    pub fn new(year: i32, month: u32, day: u32) -> Option<Self> {
        NaiveDate::from_ymd_opt(year, month, day).map(Self::from)
    }

    fn as_naive(self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year, self.month, self.day)
            .expect("CalendarDate is always constructed from a valid date")
    }
}

impl From<NaiveDate> for CalendarDate {
    fn from(value: NaiveDate) -> Self {
        Self {
            year: value.year(),
            month: value.month(),
            day: value.day(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CalendarDay {
    pub date: CalendarDate,
    pub outside_month: bool,
    pub weekend: bool,
}

pub fn month_grid(year: i32, month: u32, monday_first: bool) -> Vec<CalendarDay> {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid calendar month");
    let offset = weekday_index(first.weekday(), monday_first);
    let start = first - Duration::days(i64::from(offset));
    (0..42)
        .map(|index| {
            let date = start + Duration::days(index);
            CalendarDay {
                date: date.into(),
                outside_month: date.month() != month,
                weekend: matches!(date.weekday(), Weekday::Sat | Weekday::Sun),
            }
        })
        .collect()
}

pub fn week_grid(selected: CalendarDate, monday_first: bool) -> Vec<CalendarDay> {
    let selected = selected.as_naive();
    let start =
        selected - Duration::days(i64::from(weekday_index(selected.weekday(), monday_first)));
    (0..7)
        .map(|index| {
            let date = start + Duration::days(index);
            CalendarDay {
                date: date.into(),
                outside_month: false,
                weekend: matches!(date.weekday(), Weekday::Sat | Weekday::Sun),
            }
        })
        .collect()
}

pub fn rolling_week_grid(start: CalendarDate) -> Vec<CalendarDay> {
    let start = start.as_naive();
    (0..7)
        .map(|index| {
            let date = start + Duration::days(index);
            CalendarDay {
                date: date.into(),
                outside_month: false,
                weekend: matches!(date.weekday(), Weekday::Sat | Weekday::Sun),
            }
        })
        .collect()
}

pub fn shift_day(date: CalendarDate, delta: i64) -> CalendarDate {
    (date.as_naive() + Duration::days(delta)).into()
}

pub fn shift_month(date: CalendarDate, delta: i32) -> CalendarDate {
    let month_index = date.year * 12 + date.month as i32 - 1 + delta;
    let year = month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) as u32 + 1;
    let last_day = days_in_month(year, month);
    CalendarDate {
        year,
        month,
        day: date.day.min(last_day),
    }
}

pub fn shift_year(date: CalendarDate, delta: i32) -> CalendarDate {
    let year = date.year + delta;
    CalendarDate {
        year,
        month: date.month,
        day: date.day.min(days_in_month(year, date.month)),
    }
}

fn weekday_index(weekday: Weekday, monday_first: bool) -> u32 {
    if monday_first {
        weekday.num_days_from_monday()
    } else {
        weekday.num_days_from_sunday()
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("valid next month");
    (next - Duration::days(1)).day()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_grid_is_six_complete_weeks_with_adjacent_dates() {
        let days = month_grid(2026, 8, true);
        assert_eq!(days.len(), 42);
        assert_eq!(days[0].date, CalendarDate::new(2026, 7, 27).unwrap());
        assert!(days[0].outside_month);
        assert_eq!(days[41].date, CalendarDate::new(2026, 9, 6).unwrap());
    }

    #[test]
    fn week_grid_respects_configured_first_day() {
        let selected = CalendarDate::new(2026, 8, 30).unwrap();
        assert_eq!(week_grid(selected, true)[0].date.day, 24);
        assert_eq!(week_grid(selected, false)[0].date.day, 30);
    }

    #[test]
    fn rolling_week_advances_one_day_at_a_time() {
        let start = CalendarDate::new(2026, 8, 30).unwrap();
        let days = rolling_week_grid(start);

        assert_eq!(days[0].date, start);
        assert_eq!(days[6].date, CalendarDate::new(2026, 9, 5).unwrap());
        assert!(days[0].weekend);
        assert!(days[6].weekend);
    }

    #[test]
    fn shift_day_crosses_month_boundaries() {
        let date = CalendarDate::new(2026, 8, 31).unwrap();
        assert_eq!(shift_day(date, 1), CalendarDate::new(2026, 9, 1).unwrap());
        assert_eq!(
            shift_day(date, -31),
            CalendarDate::new(2026, 7, 31).unwrap()
        );
    }

    #[test]
    fn month_navigation_clamps_end_of_month() {
        let january = CalendarDate::new(2025, 1, 31).unwrap();
        assert_eq!(
            shift_month(january, 1),
            CalendarDate::new(2025, 2, 28).unwrap()
        );
    }

    #[test]
    fn year_navigation_clamps_leap_day() {
        let leap_day = CalendarDate::new(2024, 2, 29).unwrap();
        assert_eq!(
            shift_year(leap_day, 1),
            CalendarDate::new(2025, 2, 28).unwrap()
        );
    }
}
