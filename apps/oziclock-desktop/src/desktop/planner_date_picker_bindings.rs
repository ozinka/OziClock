use std::{cell::RefCell, rc::Rc};

use chrono::{Datelike, Months, NaiveDate, Utc};
use oziclock_storage::AppSettings;
use slint::{ComponentHandle, ModelRc, VecModel};

use super::{PlannerWindow, ReminderCalendarDayData, calendar_local_now};

pub(super) fn wire_date_picker_bindings(
    planner_window: &PlannerWindow,
    shared_settings: Rc<RefCell<AppSettings>>,
) {
    let calendar_model = Rc::new(VecModel::from(Vec::<ReminderCalendarDayData>::new()));
    planner_window.set_reminder_calendar_days(ModelRc::from(calendar_model.clone()));
    let calendar_anchor = Rc::new(RefCell::new(Utc::now().date_naive()));

    let planner_for_open = planner_window.as_weak();
    let settings_for_open = shared_settings.clone();
    let model_for_open = calendar_model.clone();
    let anchor_for_open = calendar_anchor.clone();
    planner_window.on_request_open_reminder_calendar(move || {
        let Some(planner) = planner_for_open.upgrade() else {
            return;
        };
        let today = calendar_local_now(&settings_for_open.borrow()).date();
        let selected = selected_date(&planner);
        let anchor = selected
            .unwrap_or(today)
            .with_day(1)
            .expect("the first day exists in every month");
        *anchor_for_open.borrow_mut() = anchor;
        refresh_date_picker(&planner, &model_for_open, anchor, selected, today);
        planner.set_reminder_calendar_visible(true);
    });

    let planner_for_navigate = planner_window.as_weak();
    let settings_for_navigate = shared_settings;
    let model_for_navigate = calendar_model;
    let anchor_for_navigate = calendar_anchor;
    planner_window.on_request_navigate_reminder_calendar(move |direction| {
        let Some(planner) = planner_for_navigate.upgrade() else {
            return;
        };
        let current = *anchor_for_navigate.borrow();
        let shifted = if direction < 0 {
            current.checked_sub_months(Months::new(1))
        } else {
            current.checked_add_months(Months::new(1))
        };
        let Some(anchor) = shifted else {
            return;
        };
        *anchor_for_navigate.borrow_mut() = anchor;
        let today = calendar_local_now(&settings_for_navigate.borrow()).date();
        refresh_date_picker(
            &planner,
            &model_for_navigate,
            anchor,
            selected_date(&planner),
            today,
        );
    });

    let planner_for_select = planner_window.as_weak();
    planner_window.on_request_select_reminder_date(move |date| {
        if let Some(planner) = planner_for_select.upgrade() {
            if planner.get_editor_modal() == 5 {
                planner.set_task_date_draft(date);
                planner.set_task_error("".into());
            } else if planner.get_editor_modal() == 6 {
                if planner.get_event_calendar_target() == 1 {
                    planner.set_event_end_date_draft(date);
                } else {
                    planner.set_event_date_draft(date);
                }
                planner.set_event_error("".into());
            } else {
                planner.set_reminder_date_draft(date);
                planner.set_reminder_error("".into());
            }
            planner.set_reminder_calendar_visible(false);
        }
    });
}

fn selected_date(planner: &PlannerWindow) -> Option<NaiveDate> {
    let selected_text = if planner.get_editor_modal() == 5 {
        planner.get_task_date_draft()
    } else if planner.get_editor_modal() == 6 {
        if planner.get_event_calendar_target() == 1 {
            planner.get_event_end_date_draft()
        } else {
            planner.get_event_date_draft()
        }
    } else {
        planner.get_reminder_date_draft()
    };
    NaiveDate::parse_from_str(&selected_text, "%Y-%m-%d").ok()
}

fn refresh_date_picker(
    planner: &PlannerWindow,
    model: &VecModel<ReminderCalendarDayData>,
    anchor: NaiveDate,
    selected: Option<NaiveDate>,
    today: NaiveDate,
) {
    model.set_vec(date_picker_days(anchor, selected, today));
    planner.set_reminder_calendar_title(anchor.format("%B %Y").to_string().into());
}

fn date_picker_days(
    anchor: NaiveDate,
    selected: Option<NaiveDate>,
    today: NaiveDate,
) -> Vec<ReminderCalendarDayData> {
    let first = anchor
        .with_day(1)
        .expect("the first day exists in every month");
    let grid_start = first - chrono::Duration::days(first.weekday().num_days_from_monday().into());
    (0..42)
        .map(|offset| {
            let date = grid_start + chrono::Duration::days(offset);
            ReminderCalendarDayData {
                label: date.day().to_string().into(),
                date: date.format("%Y-%m-%d").to_string().into(),
                in_month: date.month() == anchor.month() && date.year() == anchor.year(),
                enabled: date >= today,
                selected: selected == Some(date),
                today: date == today,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rem_11_date_picker_is_six_monday_first_weeks_with_past_dates_disabled() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
        let selected = NaiveDate::from_ymd_opt(2026, 9, 14).unwrap();
        let days = date_picker_days(today, Some(selected), today);

        assert_eq!(days.len(), 42);
        assert_eq!(days.first().unwrap().date.as_str(), "2026-08-31");
        assert_eq!(days.last().unwrap().date.as_str(), "2026-10-11");
        assert!(!days[7].enabled);
        assert!(days[8].today);
        assert!(days[14].selected);
        assert!(!days.first().unwrap().in_month);
        assert!(days[8].in_month);
    }
}
