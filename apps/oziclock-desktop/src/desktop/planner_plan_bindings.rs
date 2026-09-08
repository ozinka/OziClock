use std::{cell::RefCell, rc::Rc, time::Duration};

use chrono::{Datelike, Months, NaiveDate, Timelike};
use oziclock_storage::AppSettings;
use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel};

use super::{
    PlanAllDayEventData, PlanEventMarkerData, PlanMonthDayData, PlanReminderMarkerData,
    PlanTaskMarkerData, PlanWeekDayData, PlanYearMonthData, PlannerWindow, calendar_local_now,
    planner_models::{plan_counts_for_date, plan_event_rows, plan_reminder_rows, plan_task_rows},
};

pub(super) struct PlanBindings {
    pub(super) reminders: Rc<VecModel<PlanReminderMarkerData>>,
    pub(super) events: Rc<VecModel<PlanEventMarkerData>>,
    pub(super) all_day_events: Rc<VecModel<PlanAllDayEventData>>,
    pub(super) week_start: Rc<RefCell<NaiveDate>>,
}

pub(super) fn wire_plan_bindings(
    planner_window: &PlannerWindow,
    shared_settings: Rc<RefCell<AppSettings>>,
) -> PlanBindings {
    let day_model = Rc::new(VecModel::from(Vec::<PlanWeekDayData>::new()));
    let task_model = Rc::new(VecModel::from(Vec::<PlanTaskMarkerData>::new()));
    let month_model = Rc::new(VecModel::from(Vec::<PlanMonthDayData>::new()));
    let year_model = Rc::new(VecModel::from(Vec::<PlanYearMonthData>::new()));
    let reminder_model = Rc::new(VecModel::from(Vec::<PlanReminderMarkerData>::new()));
    let event_model = Rc::new(VecModel::from(Vec::<PlanEventMarkerData>::new()));
    let all_day_event_model = Rc::new(VecModel::from(Vec::<PlanAllDayEventData>::new()));
    planner_window.set_plan_week_days(ModelRc::from(day_model.clone()));
    planner_window.set_plan_tasks(ModelRc::from(task_model.clone()));
    planner_window.set_plan_month_days(ModelRc::from(month_model.clone()));
    planner_window.set_plan_year_months(ModelRc::from(year_model.clone()));
    planner_window.set_plan_reminders(ModelRc::from(reminder_model.clone()));
    planner_window.set_plan_events(ModelRc::from(event_model.clone()));
    planner_window.set_plan_all_day_events(ModelRc::from(all_day_event_model.clone()));

    let week_start = Rc::new(RefCell::new(current_week_start(&shared_settings.borrow())));
    refresh_plan_week(
        planner_window,
        &day_model,
        &task_model,
        &reminder_model,
        (&event_model, &all_day_event_model),
        &shared_settings.borrow(),
        *week_start.borrow(),
    );
    refresh_plan_overview(
        planner_window,
        &month_model,
        &year_model,
        &shared_settings.borrow(),
        calendar_local_now(&shared_settings.borrow()).date(),
    );

    let planner_for_plan_week = planner_window.as_weak();
    let settings_for_plan_week = shared_settings.clone();
    let days_for_plan_week = day_model.clone();
    let reminders_for_plan_week = reminder_model.clone();
    let tasks_for_plan_week = task_model.clone();
    let month_for_plan = month_model.clone();
    let year_for_plan = year_model.clone();
    let events_for_plan_week = event_model.clone();
    let all_day_events_for_plan_week = all_day_event_model.clone();
    let start_for_plan_week = week_start.clone();
    planner_window.on_request_plan_week(move |direction| {
        let Some(planner) = planner_for_plan_week.upgrade() else {
            return;
        };
        let view = planner.get_selected_view();
        let current = *start_for_plan_week.borrow();
        let today = calendar_local_now(&settings_for_plan_week.borrow()).date();
        let start = if view == 0 {
            if direction == 0 {
                current_week_start(&settings_for_plan_week.borrow())
            } else {
                current + chrono::Duration::days(i64::from(direction.signum()) * 7)
            }
        } else if view == 1 {
            let base = if direction == 0 {
                today
            } else if direction >= 100 {
                today.with_month(direction as u32 - 100).unwrap_or(today)
            } else {
                current
            };
            let shifted = if (-1..=1).contains(&direction) && direction != 0 {
                if direction < 0 {
                    base.checked_sub_months(Months::new(1))
                } else {
                    base.checked_add_months(Months::new(1))
                }
                .unwrap_or(base)
            } else {
                base
            };
            shifted.with_day(1).unwrap_or(shifted)
        } else {
            let base = if direction == 0 { today } else { current };
            let shifted = if direction < 0 {
                base.checked_sub_months(Months::new(12))
            } else if direction > 0 {
                base.checked_add_months(Months::new(12))
            } else {
                Some(base)
            }
            .unwrap_or(base);
            shifted
                .with_month(1)
                .and_then(|date| date.with_day(1))
                .unwrap_or(shifted)
        };
        *start_for_plan_week.borrow_mut() = start;
        planner.set_selected_plan_date("".into());
        planner.set_selected_plan_hour(-1);
        planner.set_selected_plan_reminder("".into());
        planner.set_selected_plan_event("".into());
        planner.set_selected_plan_task("".into());
        planner.set_selected_plan_title("".into());
        planner.set_selected_plan_time("".into());
        if view == 0 {
            refresh_plan_week(
                &planner,
                &days_for_plan_week,
                &tasks_for_plan_week,
                &reminders_for_plan_week,
                (&events_for_plan_week, &all_day_events_for_plan_week),
                &settings_for_plan_week.borrow(),
                start,
            );
        } else {
            refresh_plan_overview(
                &planner,
                &month_for_plan,
                &year_for_plan,
                &settings_for_plan_week.borrow(),
                start,
            );
        }
    });

    schedule_plan_refresh(
        Rc::new(Timer::default()),
        planner_window.as_weak(),
        PlanRefreshModels {
            days: day_model,
            tasks: task_model,
            month_days: month_model,
            year_months: year_model,
            reminders: reminder_model.clone(),
            events: event_model.clone(),
            all_day_events: all_day_event_model.clone(),
        },
        shared_settings,
        week_start.clone(),
    );

    PlanBindings {
        reminders: reminder_model,
        events: event_model,
        all_day_events: all_day_event_model,
        week_start,
    }
}

fn current_week_start(settings: &AppSettings) -> NaiveDate {
    let today = calendar_local_now(settings).date();
    today - chrono::Duration::days(today.weekday().num_days_from_monday().into())
}

fn refresh_plan_overview(
    planner: &PlannerWindow,
    month_model: &VecModel<PlanMonthDayData>,
    year_model: &VecModel<PlanYearMonthData>,
    settings: &AppSettings,
    anchor: NaiveDate,
) {
    let today = calendar_local_now(settings).date();
    let first = anchor.with_day(1).expect("month has a first day");
    let grid_start = first - chrono::Duration::days(first.weekday().num_days_from_monday().into());
    month_model.set_vec(
        (0..42)
            .map(|offset| {
                let date = grid_start + chrono::Duration::days(offset);
                let (event_count, task_count) = plan_counts_for_date(settings, date);
                PlanMonthDayData {
                    label: date.day().to_string().into(),
                    date: date.format("%Y-%m-%d").to_string().into(),
                    in_month: date.month() == anchor.month(),
                    today: date == today,
                    event_count,
                    task_count,
                }
            })
            .collect::<Vec<_>>(),
    );
    year_model.set_vec(
        (1..=12)
            .map(|month| {
                let mut event_count = 0;
                let mut task_count = 0;
                let mut date =
                    NaiveDate::from_ymd_opt(anchor.year(), month, 1).expect("valid month");
                while date.month() == month {
                    let counts = plan_counts_for_date(settings, date);
                    event_count += counts.0;
                    task_count += counts.1;
                    date += chrono::Duration::days(1);
                }
                PlanYearMonthData {
                    label: NaiveDate::from_ymd_opt(anchor.year(), month, 1)
                        .unwrap()
                        .format("%B")
                        .to_string()
                        .into(),
                    month: month as i32,
                    current: today.year() == anchor.year() && today.month() == month,
                    event_count,
                    task_count,
                }
            })
            .collect::<Vec<_>>(),
    );
    planner.set_plan_week_title(
        if planner.get_selected_view() == 1 {
            anchor.format("%B %Y")
        } else {
            anchor.format("%Y")
        }
        .to_string()
        .into(),
    );
}

fn refresh_plan_week(
    planner: &PlannerWindow,
    day_model: &VecModel<PlanWeekDayData>,
    task_model: &VecModel<PlanTaskMarkerData>,
    reminder_model: &VecModel<PlanReminderMarkerData>,
    event_models: (
        &VecModel<PlanEventMarkerData>,
        &VecModel<PlanAllDayEventData>,
    ),
    settings: &AppSettings,
    start: NaiveDate,
) {
    let local_now = calendar_local_now(settings);
    let today = local_now.date();
    day_model.set_vec(
        (0..7)
            .map(|offset| {
                let date = start + chrono::Duration::days(offset);
                PlanWeekDayData {
                    label: date.format("%a %-d").to_string().into(),
                    date: date.format("%Y-%m-%d").to_string().into(),
                    weekend: date.weekday().num_days_from_monday() >= 5,
                    today: date == today,
                }
            })
            .collect::<Vec<_>>(),
    );
    let reminder_rows = plan_reminder_rows(settings, start);
    let selected_reminder = planner.get_selected_plan_reminder();
    if !selected_reminder.is_empty()
        && !reminder_rows
            .iter()
            .any(|row| row.id.as_str() == selected_reminder.as_str())
    {
        planner.set_selected_plan_date("".into());
        planner.set_selected_plan_hour(-1);
        planner.set_selected_plan_reminder("".into());
        planner.set_selected_plan_title("".into());
        planner.set_selected_plan_time("".into());
    }
    reminder_model.set_vec(reminder_rows);
    task_model.set_vec(plan_task_rows(settings, start));
    let (event_rows, all_day_rows) = plan_event_rows(settings, start);
    let selected_event = planner.get_selected_plan_event();
    if !selected_event.is_empty()
        && !event_rows
            .iter()
            .any(|row| row.id.as_str() == selected_event.as_str())
        && !all_day_rows
            .iter()
            .any(|row| row.id.as_str() == selected_event.as_str())
    {
        planner.set_selected_plan_date("".into());
        planner.set_selected_plan_event("".into());
        planner.set_selected_plan_title("".into());
        planner.set_selected_plan_time("".into());
    }
    event_models.0.set_vec(event_rows);
    event_models.1.set_vec(all_day_rows);
    let current_day = today.signed_duration_since(start).num_days();
    let current_minute =
        i32::try_from(local_now.time().num_seconds_from_midnight() / 60).unwrap_or_default();
    if (0..7).contains(&current_day) {
        planner.set_current_plan_day(current_day as i32);
        planner.set_current_plan_minute(current_minute);
        planner.set_current_plan_time(local_now.time().format("%H:%M").to_string().into());
    } else {
        planner.set_current_plan_day(-1);
        planner.set_current_plan_minute(-1);
        planner.set_current_plan_time("".into());
    }
    let title = if start == current_week_start(settings) {
        "This week".to_owned()
    } else {
        format!(
            "{} – {}",
            start.format("%-d %b"),
            (start + chrono::Duration::days(6)).format("%-d %b")
        )
    };
    planner.set_plan_week_title(title.into());
}

#[derive(Clone)]
struct PlanRefreshModels {
    days: Rc<VecModel<PlanWeekDayData>>,
    tasks: Rc<VecModel<PlanTaskMarkerData>>,
    month_days: Rc<VecModel<PlanMonthDayData>>,
    year_months: Rc<VecModel<PlanYearMonthData>>,
    reminders: Rc<VecModel<PlanReminderMarkerData>>,
    events: Rc<VecModel<PlanEventMarkerData>>,
    all_day_events: Rc<VecModel<PlanAllDayEventData>>,
}

fn schedule_plan_refresh(
    timer: Rc<Timer>,
    planner: slint::Weak<PlannerWindow>,
    models: PlanRefreshModels,
    settings: Rc<RefCell<AppSettings>>,
    start: Rc<RefCell<NaiveDate>>,
) {
    let next_timer = timer.clone();
    let next_models = models.clone();
    let next_settings = settings.clone();
    let next_start = start.clone();
    timer.start(TimerMode::SingleShot, Duration::from_secs(60), move || {
        if let Some(planner) = planner.upgrade() {
            if planner.get_selected_view() == 0 {
                refresh_plan_week(
                    &planner,
                    &models.days,
                    &models.tasks,
                    &models.reminders,
                    (&models.events, &models.all_day_events),
                    &settings.borrow(),
                    *start.borrow(),
                );
            } else {
                refresh_plan_overview(
                    &planner,
                    &models.month_days,
                    &models.year_months,
                    &settings.borrow(),
                    *start.borrow(),
                );
            }
        }
        schedule_plan_refresh(
            next_timer.clone(),
            planner.clone(),
            next_models.clone(),
            next_settings.clone(),
            next_start.clone(),
        );
    });
}
