use super::planner_models::{plan_event_rows, plan_task_rows};
use super::{CalendarAgendaData, CalendarDayData, CalendarMonthData, CalendarWindow};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Timelike};
use chrono_tz::Tz;
use oziclock_app::calendar::{
    CalendarDate, CalendarDay, month_grid, rolling_week_grid, shift_day, shift_month, shift_year,
};
use oziclock_storage::{AlarmSchedule, AppSettings, ReminderRecurrence, ReminderSchedule};
use slint::{ModelRc, VecModel};

#[derive(Clone, Copy)]
enum CalendarItemKind {
    Event,
    Reminder,
    Task,
    Alarm,
}

impl CalendarItemKind {
    fn label(self) -> &'static str {
        match self {
            Self::Event => "Event",
            Self::Reminder => "Reminder",
            Self::Task => "Task",
            Self::Alarm => "Alarm",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Event => 0,
            Self::Reminder => 1,
            Self::Task => 2,
            Self::Alarm => 3,
        }
    }
}

#[derive(Clone)]
struct CalendarItem {
    kind: CalendarItemKind,
    title: String,
    time: String,
}

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
// Keep ample scrollable runway so the logical week can advance continuously
// across month and year boundaries without the native ScrollView clamping.
const WEEK_CONTENT_HOURS: i32 = 24 * 42;
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
    let week = rolling_week_grid(state.cursor);

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

pub(super) fn refresh_calendar_items(
    window: &CalendarWindow,
    state: &CalendarState,
    today: CalendarDate,
    settings: &AppSettings,
) {
    let colors = super::planner_appearance::calendar_indicator_colors(settings);
    window.set_event_color(colors[0]);
    window.set_reminder_color(colors[1]);
    window.set_task_color(colors[2]);
    window.set_alarm_color(colors[3]);
    window.set_days(day_model_with_items(
        month_grid(state.cursor.year, state.cursor.month, state.monday_first),
        state,
        today,
        settings,
    ));
    window.set_week_days(day_model_with_items(
        rolling_week_grid(state.cursor),
        state,
        today,
        settings,
    ));
    window.set_months(month_model_with_items(state, today, settings));
    let selected_items = calendar_items_for_date(settings, state.selected);
    window.set_selected_items(ModelRc::new(VecModel::from(
        selected_items
            .into_iter()
            .map(|item| CalendarAgendaData {
                kind: item.kind.label().into(),
                title: item.title.into(),
                time: item.time.into(),
                color: colors[item.kind.index()],
            })
            .collect::<Vec<_>>(),
    )));
}

fn month_model_with_items(
    state: &CalendarState,
    today: CalendarDate,
    settings: &AppSettings,
) -> ModelRc<CalendarMonthData> {
    ModelRc::new(VecModel::from(
        (1..=12)
            .map(|month| CalendarMonthData {
                name: MONTHS[month as usize - 1].into(),
                current: today.year == state.cursor.year && today.month == month,
                days: day_model_with_items(
                    month_grid(state.cursor.year, month, state.monday_first),
                    state,
                    today,
                    settings,
                ),
            })
            .collect::<Vec<_>>(),
    ))
}

fn day_model_with_items(
    days: Vec<CalendarDay>,
    state: &CalendarState,
    today: CalendarDate,
    settings: &AppSettings,
) -> ModelRc<CalendarDayData> {
    ModelRc::new(VecModel::from(
        days.into_iter()
            .map(|day| {
                let items = calendar_items_for_date(settings, day.date);
                CalendarDayData {
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
                    has_event: items
                        .iter()
                        .any(|item| matches!(item.kind, CalendarItemKind::Event)),
                    has_reminder: items
                        .iter()
                        .any(|item| matches!(item.kind, CalendarItemKind::Reminder)),
                    has_task: items
                        .iter()
                        .any(|item| matches!(item.kind, CalendarItemKind::Task)),
                    has_alarm: items
                        .iter()
                        .any(|item| matches!(item.kind, CalendarItemKind::Alarm)),
                }
            })
            .collect::<Vec<_>>(),
    ))
}

fn calendar_items_for_date(settings: &AppSettings, date: CalendarDate) -> Vec<CalendarItem> {
    let target =
        NaiveDate::from_ymd_opt(date.year, date.month, date.day).expect("calendar date is valid");
    let week_start =
        target - chrono::Duration::days(target.weekday().num_days_from_monday().into());
    let target_id = target.format("%Y-%m-%d").to_string();
    let (timed_events, all_day_events) = plan_event_rows(settings, week_start);
    let mut items = timed_events
        .into_iter()
        .filter(|item| item.date.as_str() == target_id)
        .map(|item| CalendarItem {
            kind: CalendarItemKind::Event,
            title: item.title.to_string(),
            time: item.time.to_string(),
        })
        .collect::<Vec<_>>();
    items.extend(
        all_day_events
            .into_iter()
            .filter(|item| item.date.as_str() == target_id)
            .map(|item| CalendarItem {
                kind: CalendarItemKind::Event,
                title: item.title.to_string(),
                time: "All day".into(),
            }),
    );
    items.extend(
        plan_task_rows(settings, week_start)
            .into_iter()
            .filter(|item| item.date.as_str() == target_id)
            .map(|item| CalendarItem {
                kind: CalendarItemKind::Task,
                title: item.title.to_string(),
                time: item.time.to_string(),
            }),
    );
    let zone = super::main_time_zone(settings)
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC);
    for reminder in settings
        .planner
        .reminders
        .iter()
        .filter(|item| item.enabled)
    {
        if let Some(time) = reminder_time_for_date(&reminder.schedule, target, zone) {
            items.push(CalendarItem {
                kind: CalendarItemKind::Reminder,
                title: reminder.title.clone(),
                time,
            });
        }
    }
    for alarm in settings.planner.alarms.iter().filter(|item| item.enabled) {
        let matches = match &alarm.schedule {
            AlarmSchedule::Once {
                local_date,
                local_time,
            } => {
                oziclock_app::reminder_time::resolve_local(local_date, local_time, &alarm.time_zone)
                    .is_some_and(|instant| instant.with_timezone(&zone).date_naive() == target)
            }
            AlarmSchedule::Weekly {
                local_time,
                weekdays,
            } => {
                let alarm_zone = alarm.time_zone.parse::<Tz>().unwrap_or(chrono_tz::UTC);
                let source = zone
                    .from_local_datetime(&target.and_hms_opt(12, 0, 0).expect("valid noon"))
                    .single()
                    .map(|item| item.with_timezone(&alarm_zone));
                source.is_some_and(|item| {
                    weekdays.contains(&(item.weekday().num_days_from_monday() as u8))
                        && oziclock_app::reminder_time::resolve_local(
                            &item.date_naive().format("%Y-%m-%d").to_string(),
                            local_time,
                            &alarm.time_zone,
                        )
                        .is_some_and(|instant| instant.with_timezone(&zone).date_naive() == target)
                })
            }
        };
        if matches {
            let time = match &alarm.schedule {
                AlarmSchedule::Once { local_time, .. }
                | AlarmSchedule::Weekly { local_time, .. } => local_time.clone(),
            };
            items.push(CalendarItem {
                kind: CalendarItemKind::Alarm,
                title: alarm.title.clone(),
                time,
            });
        }
    }
    items.sort_by(|left, right| {
        left.time
            .cmp(&right.time)
            .then(left.title.cmp(&right.title))
    });
    items
}

fn reminder_time_for_date(
    schedule: &ReminderSchedule,
    target: NaiveDate,
    zone: Tz,
) -> Option<String> {
    match schedule {
        ReminderSchedule::Absolute { at_utc, .. } => DateTime::parse_from_rfc3339(at_utc)
            .ok()
            .map(|value| value.with_timezone(&zone))
            .filter(|value| value.date_naive() == target)
            .map(|value| value.format("%H:%M").to_string()),
        ReminderSchedule::ImportantDate { month, day } => (target.month() == u32::from(*month)
            && target.day() == u32::from(*day))
        .then(|| "All day".into()),
        ReminderSchedule::Recurring {
            recurrence,
            local_time,
            source_time_zone,
            next_at_utc,
        } => {
            let source_zone = source_time_zone.parse::<Tz>().ok()?;
            let anchor = DateTime::parse_from_rfc3339(next_at_utc)
                .ok()?
                .with_timezone(&source_zone)
                .date_naive();
            (-1..=1).find_map(|offset| {
                let source_date = target.checked_add_signed(chrono::Duration::days(offset))?;
                (source_date >= anchor && recurrence_matches_date(recurrence, source_date))
                    .then_some(())?;
                let due = oziclock_app::reminder_time::resolve_local(
                    &source_date.format("%Y-%m-%d").to_string(),
                    local_time,
                    source_time_zone,
                )?
                .with_timezone(&zone);
                (due.date_naive() == target).then(|| due.format("%H:%M").to_string())
            })
        }
    }
}

fn recurrence_matches_date(recurrence: &ReminderRecurrence, date: NaiveDate) -> bool {
    match recurrence {
        ReminderRecurrence::Daily => true,
        ReminderRecurrence::Weekly { weekdays } => {
            weekdays.contains(&(date.weekday().num_days_from_monday() as u8))
        }
        ReminderRecurrence::Monthly { day } => {
            date.day() == u32::from(*day).min(days_in_month(date.year(), date.month()))
        }
        ReminderRecurrence::Yearly { month, day } => {
            date.month() == u32::from(*month)
                && date.day() == u32::from(*day).min(days_in_month(date.year(), u32::from(*month)))
        }
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("valid next month");
    (next - chrono::Duration::days(1)).day()
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
    fn cal_11_recurring_reminder_marks_its_local_calendar_day() {
        let schedule = ReminderSchedule::Recurring {
            recurrence: ReminderRecurrence::Weekly { weekdays: vec![0] },
            local_time: "09:30".into(),
            source_time_zone: "Europe/Kyiv".into(),
            next_at_utc: "2026-09-07T06:30:00Z".into(),
        };
        let monday = NaiveDate::from_ymd_opt(2026, 9, 14).unwrap();
        assert_eq!(
            reminder_time_for_date(&schedule, monday, chrono_tz::Europe::Kyiv),
            Some("09:30".into())
        );
        assert_eq!(
            reminder_time_for_date(
                &schedule,
                monday + chrono::Duration::days(1),
                chrono_tz::Europe::Kyiv
            ),
            None
        );
    }
}
