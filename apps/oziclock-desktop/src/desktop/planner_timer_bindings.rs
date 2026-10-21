use std::{cell::RefCell, collections::VecDeque, rc::Rc, time::Duration};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use oziclock_app::planner::{PlannerCommand, execute_planner_command};
use oziclock_storage::{AppSettings, PlannerId, PlannerTimer, TimerState};
use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{ComponentHandle, Model, ModelRc, Timer, TimerMode, VecModel};

use super::{
    AppWindow, AuxiliaryFocusPolicy, PlannerTimerData, PlannerWindow, TimerAttentionWindow,
    alert_sound::AlertSound,
    delivery_feedback,
    planner_inputs::{
        adjust_timer_part, duration_parts, mask_timer_part, parse_timer_duration, parse_timer_part,
        store_timer_part,
    },
    planner_models::{planner_timer_rows, timer_display_title},
    position_calendar_window, show_auxiliary_window,
};

pub(super) fn wire_timer_bindings(
    alert_sound: AlertSound,
    planner_window: &PlannerWindow,
    attention_window: &TimerAttentionWindow,
    owner: &AppWindow,
    shared_settings: Rc<RefCell<AppSettings>>,
) {
    let attention_for_close = attention_window.as_weak();
    attention_window.window().on_close_requested(move || {
        if let Some(window) = attention_for_close.upgrade() {
            window.set_pulse_active(false);
        }
        slint::CloseRequestResponse::HideWindow
    });

    let timer_model = Rc::new(VecModel::from(planner_timer_rows(
        &shared_settings.borrow(),
    )));
    planner_window.set_timers(ModelRc::from(timer_model.clone()));
    let attention_queue = Rc::new(RefCell::new(pending_timer_attention_items(
        &shared_settings.borrow(),
    )));
    let presenter = TimerAttentionPresenter {
        window: attention_window.as_weak(),
        planner: planner_window.as_weak(),
        owner: owner.as_weak(),
        queue: attention_queue.clone(),
    };
    let presenter_for_view = presenter.clone();
    planner_window.on_request_refresh_timer_attention(move || presenter_for_view.refresh());
    let presenter_for_events = presenter.clone();
    planner_window
        .window()
        .on_winit_window_event(move |_, event| {
            if matches!(
                event,
                WindowEvent::Focused(_) | WindowEvent::Occluded(_) | WindowEvent::CloseRequested
            ) {
                // Let the native/backend focus and visibility state settle first.
                let presenter = presenter_for_events.clone();
                Timer::single_shot(Duration::ZERO, move || presenter.refresh());
            }
            EventResult::Propagate
        });
    let presenter_for_inline_dismiss = presenter.clone();
    planner_window.on_request_dismiss_timer_attention(move |id| {
        if presenter_for_inline_dismiss.is_inline(id.as_str())
            && let Some(window) = presenter_for_inline_dismiss.window.upgrade()
        {
            window.invoke_request_dismiss();
        }
    });
    let presenter_for_inline_restart = presenter.clone();
    planner_window.on_request_restart_timer_attention(move |id| {
        if presenter_for_inline_restart.is_inline(id.as_str())
            && let Some(window) = presenter_for_inline_restart.window.upgrade()
        {
            window.invoke_request_restart();
        }
    });

    let queue_for_timer_dismiss = attention_queue.clone();
    let sound_for_timer_dismiss = alert_sound.clone();
    let settings_for_timer_dismiss = shared_settings.clone();
    let model_for_timer_dismiss = timer_model.clone();
    let presenter_for_timer_dismiss = presenter.clone();
    attention_window.on_request_dismiss(move || {
        let Some(item) = queue_for_timer_dismiss.borrow().front().cloned() else {
            return;
        };
        let mut settings = settings_for_timer_dismiss.borrow_mut();
        if commit_timer_attention(
            &mut settings,
            PlannerCommand::DismissTimer { id: item.timer_id },
            |updated| oziclock_storage::save(updated).is_ok(),
        ) {
            sound_for_timer_dismiss.stop();
            model_for_timer_dismiss.set_vec(planner_timer_rows(&settings));
            queue_for_timer_dismiss.borrow_mut().pop_front();
            presenter_for_timer_dismiss.refresh();
        }
    });

    let queue_for_timer_restart = attention_queue.clone();
    let sound_for_timer_restart = alert_sound.clone();
    let settings_for_timer_restart = shared_settings.clone();
    let model_for_timer_restart = timer_model.clone();
    let presenter_for_timer_restart = presenter.clone();
    attention_window.on_request_restart(move || {
        let Some(item) = queue_for_timer_restart.borrow().front().cloned() else {
            return;
        };
        let mut settings = settings_for_timer_restart.borrow_mut();
        if commit_timer_attention(
            &mut settings,
            PlannerCommand::RestartTimer {
                id: item.timer_id,
                started_at_utc: Utc::now().to_rfc3339(),
            },
            |updated| oziclock_storage::save(updated).is_ok(),
        ) {
            sound_for_timer_restart.stop();
            model_for_timer_restart.set_vec(planner_timer_rows(&settings));
            queue_for_timer_restart.borrow_mut().pop_front();
            presenter_for_timer_restart.refresh();
        }
    });
    presenter.refresh();

    let planner_for_edit_timer_part = planner_window.as_weak();
    let settings_for_edit_timer = shared_settings.clone();
    planner_window.on_request_edit_timer_part(move |part, text| {
        let Some(masked) = mask_timer_part(&text, part) else {
            return;
        };
        let Some(planner) = planner_for_edit_timer_part.upgrade() else {
            return;
        };
        match part {
            0 => planner.set_timer_days_draft(masked.clone().into()),
            1 => planner.set_timer_hours_draft(masked.clone().into()),
            2 => planner.set_timer_minutes_draft(masked.clone().into()),
            3 => planner.set_timer_seconds_draft(masked.clone().into()),
            _ => return,
        }
        let Some(value) = parse_timer_part(&masked, part) else {
            return;
        };
        let mut settings = settings_for_edit_timer.borrow_mut();
        store_timer_part(&mut settings, part, value);
        let _ = oziclock_storage::save(&settings);
    });

    let planner_for_adjust_timer = planner_window.as_weak();
    let settings_for_adjust_timer = shared_settings.clone();
    planner_window.on_request_adjust_timer_part(move |part, direction| {
        let Some(planner) = planner_for_adjust_timer.upgrade() else {
            return;
        };
        let text = match part {
            0 => planner.get_timer_days_draft(),
            1 => planner.get_timer_hours_draft(),
            2 => planner.get_timer_minutes_draft(),
            3 => planner.get_timer_seconds_draft(),
            _ => return,
        };
        let Some(value) = adjust_timer_part(&text, part, direction) else {
            return;
        };
        let mut settings = settings_for_adjust_timer.borrow_mut();
        store_timer_part(&mut settings, part, value);
        match part {
            0 => planner.set_timer_days_draft(value.to_string().into()),
            1 => planner.set_timer_hours_draft(value.to_string().into()),
            2 => planner.set_timer_minutes_draft(value.to_string().into()),
            _ => planner.set_timer_seconds_draft(value.to_string().into()),
        }
        let _ = oziclock_storage::save(&settings);
    });

    let planner_for_add_timer = planner_window.as_weak();
    let settings_for_add_timer = shared_settings.clone();
    let timer_model_for_add_timer = timer_model.clone();
    let queue_for_add_timer = attention_queue.clone();
    let presenter_for_add_timer = presenter.clone();
    planner_window.on_request_add_timer(move |title, days, hours, minutes, seconds, repeat| {
        let Some(planner) = planner_for_add_timer.upgrade() else {
            return;
        };
        planner.set_timer_error("".into());
        let title = title.trim();
        if title.is_empty() {
            planner.set_timer_error("Enter a timer name.".into());
            return;
        }
        let Some((days, hours, minutes, seconds)) =
            parse_timer_duration(&days, &hours, &minutes, &seconds)
        else {
            planner.set_timer_error("Enter a valid duration.".into());
            return;
        };
        let duration_seconds = u64::from(days) * 86_400
            + u64::from(hours) * 3_600
            + u64::from(minutes) * 60
            + u64::from(seconds);
        if duration_seconds == 0 {
            planner.set_timer_error("Duration must be longer than zero.".into());
            return;
        }
        let repeat_count = if !repeat {
            Some(0)
        } else if planner.get_timer_repeat_unlimited() {
            None
        } else {
            let Ok(count) = planner.get_timer_repeat_count().trim().parse::<u32>() else {
                planner.set_timer_error("Enter a positive repeat count or choose ∞.".into());
                return;
            };
            if count == 0 {
                planner.set_timer_error("Repeat count must be greater than zero.".into());
                return;
            }
            Some(count)
        };
        let mut settings = settings_for_add_timer.borrow_mut();
        let mut updated = settings.clone();
        let editing = planner.get_editing_timer();
        let editing_id = PlannerId::new(editing.to_string());
        let changed = if editing.is_empty() {
            let id = PlannerId::new(format!(
                "timer-{}",
                Utc::now().timestamp_nanos_opt().unwrap_or_default()
            ))
            .expect("generated timer id is valid");
            updated.planner.timers.push(PlannerTimer {
                id,
                title: title.to_owned(),
                duration_seconds,
                remaining_seconds: duration_seconds,
                state: TimerState::Idle,
                repeat,
                repeat_count,
                repeats_remaining: repeat_count,
                started_at_utc: None,
                attention_pending: false,
                attention_triggered_at_utc: None,
            });
            true
        } else {
            let Some(id) = editing_id.clone() else {
                return;
            };
            execute_planner_command(
                &mut updated.planner,
                PlannerCommand::UpdateTimer {
                    id,
                    title: title.to_owned(),
                    duration_seconds,
                    repeat,
                    repeat_count,
                },
            )
        };
        if !changed {
            return;
        }
        updated.timer_draft_days = days;
        updated.timer_draft_hours = hours;
        updated.timer_draft_minutes = minutes;
        updated.timer_draft_seconds = seconds;
        if let Err(error) = oziclock_storage::save(&updated) {
            planner.set_timer_error(format!("Save failed: {error}").into());
            return;
        }
        *settings = updated;
        timer_model_for_add_timer.set_vec(planner_timer_rows(&settings));
        if let Some(id) = editing_id {
            queue_for_add_timer
                .borrow_mut()
                .retain(|item| item.timer_id != id);
            presenter_for_add_timer.refresh();
        }
        planner.set_timer_days_draft(days.to_string().into());
        planner.set_timer_hours_draft(hours.to_string().into());
        planner.set_timer_minutes_draft(minutes.to_string().into());
        planner.set_timer_seconds_draft(seconds.to_string().into());
        planner.set_editing_timer("".into());
        planner.set_selected_timer("".into());
        planner.set_timer_title_draft("Timer".into());
        planner.set_timer_repeat(false);
        planner.set_timer_repeat_count("1".into());
        planner.set_timer_repeat_unlimited(false);
        planner.set_editor_modal(-1);
    });

    let settings_for_edit_timer_item = shared_settings.clone();
    let planner_for_edit_timer_item = planner_window.as_weak();
    planner_window.on_request_edit_timer(move |timer_id| {
        let settings = settings_for_edit_timer_item.borrow();
        let Some(timer) = settings
            .planner
            .timers
            .iter()
            .find(|timer| timer.id.to_string() == timer_id.as_str())
        else {
            return;
        };
        let Some(planner) = planner_for_edit_timer_item.upgrade() else {
            return;
        };
        let (days, hours, minutes, seconds) = duration_parts(timer.duration_seconds);
        planner.set_editing_timer(timer_id);
        planner.set_timer_error("".into());
        planner.set_timer_title_draft(timer.title.clone().into());
        planner.set_timer_repeat(timer.repeat);
        planner.set_timer_repeat_count(timer.repeat_count.unwrap_or(1).to_string().into());
        planner.set_timer_repeat_unlimited(timer.repeat && timer.repeat_count.is_none());
        planner.set_timer_days_draft(days.to_string().into());
        planner.set_timer_hours_draft(hours.to_string().into());
        planner.set_timer_minutes_draft(minutes.to_string().into());
        planner.set_timer_seconds_draft(seconds.to_string().into());
    });

    let settings_for_reset_timer = shared_settings.clone();
    let model_for_reset_timer = timer_model.clone();
    let queue_for_reset_timer = attention_queue.clone();
    let presenter_for_reset_timer = presenter.clone();
    planner_window.on_request_reset_timer(move |timer_id| {
        let Some(id) = PlannerId::new(timer_id.to_string()) else {
            return;
        };
        let mut settings = settings_for_reset_timer.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::ResetTimer { id: id.clone() },
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            model_for_reset_timer.set_vec(planner_timer_rows(&settings));
            queue_for_reset_timer
                .borrow_mut()
                .retain(|item| item.timer_id != id);
            presenter_for_reset_timer.refresh();
        }
    });

    let settings_for_delete_timer = shared_settings.clone();
    let model_for_delete_timer = timer_model.clone();
    let planner_for_delete_timer = planner_window.as_weak();
    let queue_for_delete_timer = attention_queue.clone();
    let presenter_for_delete_timer = presenter.clone();
    planner_window.on_request_delete_timer(move |timer_id| {
        let Some(id) = PlannerId::new(timer_id.to_string()) else {
            return;
        };
        let mut settings = settings_for_delete_timer.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(
            &mut updated.planner,
            PlannerCommand::DeleteTimer { id: id.clone() },
        ) && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            model_for_delete_timer.set_vec(planner_timer_rows(&settings));
            queue_for_delete_timer
                .borrow_mut()
                .retain(|item| item.timer_id != id);
            presenter_for_delete_timer.refresh();
            if let Some(planner) = planner_for_delete_timer.upgrade() {
                if planner.get_expanded_timer() == timer_id {
                    planner.set_expanded_timer("".into());
                }
                if planner.get_selected_timer() == timer_id {
                    planner.set_selected_timer("".into());
                }
                if planner.get_editing_timer() == timer_id {
                    planner.set_editing_timer("".into());
                }
            }
        }
    });

    let settings_for_toggle_timer = shared_settings.clone();
    let timer_model_for_toggle_timer = timer_model.clone();
    let queue_for_toggle_timer = attention_queue.clone();
    let presenter_for_toggle_timer = presenter.clone();
    planner_window.on_request_toggle_timer(move |timer_id| {
        let Some(id) = PlannerId::new(timer_id.to_string()) else {
            return;
        };
        let mut settings = settings_for_toggle_timer.borrow_mut();
        let now = Utc::now();
        let command = settings
            .planner
            .timers
            .iter()
            .find(|timer| timer.id == id)
            .map(|timer| {
                if timer.state == TimerState::Running {
                    PlannerCommand::PauseTimer {
                        id: id.clone(),
                        remaining_seconds: oziclock_app::timer_time::remaining_seconds(timer, now),
                    }
                } else {
                    PlannerCommand::StartTimer {
                        id: id.clone(),
                        started_at_utc: now.to_rfc3339(),
                    }
                }
            });
        let mut updated = settings.clone();
        if let Some(command) = command
            && execute_planner_command(&mut updated.planner, command)
            && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            timer_model_for_toggle_timer.set_vec(planner_timer_rows(&settings));
            queue_for_toggle_timer
                .borrow_mut()
                .retain(|item| item.timer_id != id);
            presenter_for_toggle_timer.refresh();
        }
    });

    schedule_planner_timer_refresh(
        alert_sound,
        Rc::new(Timer::default()),
        timer_model,
        shared_settings,
        presenter,
    );
}

#[derive(Clone)]
struct TimerAttentionItem {
    timer_id: PlannerId,
    title: String,
    triggered_at: String,
}

fn commit_timer_attention(
    settings: &mut AppSettings,
    command: PlannerCommand,
    persist: impl FnOnce(&AppSettings) -> bool,
) -> bool {
    let mut updated = settings.clone();
    if !execute_planner_command(&mut updated.planner, command) || !persist(&updated) {
        return false;
    }
    *settings = updated;
    true
}

fn pending_timer_attention_items(settings: &AppSettings) -> VecDeque<TimerAttentionItem> {
    oziclock_app::timer_time::pending_attention(&settings.planner)
        .into_iter()
        .filter_map(|id| {
            settings
                .planner
                .timers
                .iter()
                .find(|timer| timer.id == id)
                .map(|timer| TimerAttentionItem {
                    timer_id: id,
                    title: timer_display_title(timer),
                    triggered_at: timer_triggered_label(timer, settings),
                })
        })
        .collect()
}

#[derive(Clone)]
struct TimerAttentionPresenter {
    window: slint::Weak<TimerAttentionWindow>,
    planner: slint::Weak<PlannerWindow>,
    owner: slint::Weak<AppWindow>,
    queue: Rc<RefCell<VecDeque<TimerAttentionItem>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimerAttentionSurface {
    Hidden,
    Inline,
    Card,
}

#[derive(Clone, Copy, Default)]
struct TimerAttentionView<'a> {
    visible: bool,
    focused: bool,
    timers_section: bool,
    editor_open: bool,
    expanded_timer: &'a str,
}

fn timer_attention_surface(
    front: Option<&str>,
    view: &TimerAttentionView<'_>,
) -> TimerAttentionSurface {
    match front {
        None => TimerAttentionSurface::Hidden,
        Some(id)
            if view.visible
                && view.focused
                && view.timers_section
                && !view.editor_open
                && !view.expanded_timer.is_empty()
                && view.expanded_timer == id =>
        {
            TimerAttentionSurface::Inline
        }
        Some(_) => TimerAttentionSurface::Card,
    }
}

impl TimerAttentionPresenter {
    fn surface(&self, item: Option<&TimerAttentionItem>) -> TimerAttentionSurface {
        let Some(item) = item else {
            return TimerAttentionSurface::Hidden;
        };
        let id = item.timer_id.to_string();
        let Some(planner) = self.planner.upgrade() else {
            return timer_attention_surface(Some(&id), &TimerAttentionView::default());
        };
        let expanded = planner.get_expanded_timer();
        let focused = planner
            .window()
            .with_winit_window(|native| native.has_focus() && native.is_minimized() != Some(true))
            .unwrap_or(false);
        timer_attention_surface(
            Some(&id),
            &TimerAttentionView {
                visible: planner.window().is_visible(),
                focused,
                timers_section: planner.get_selected_section() == 2,
                editor_open: planner.get_editor_modal() >= 0,
                expanded_timer: expanded.as_str(),
            },
        )
    }

    fn is_inline(&self, id: &str) -> bool {
        let front = self.queue.borrow().front().cloned();
        front
            .as_ref()
            .is_some_and(|item| item.timer_id.to_string() == id)
            && self.surface(front.as_ref()) == TimerAttentionSurface::Inline
    }

    fn refresh(&self) {
        let front = self.queue.borrow().front().cloned();
        let surface = self.surface(front.as_ref());
        if let Some(planner) = self.planner.upgrade() {
            if surface == TimerAttentionSurface::Inline
                && let Some(item) = &front
            {
                planner.set_inline_timer_triggered_at(item.triggered_at.clone().into());
                planner.set_inline_timer_attention(item.timer_id.to_string().into());
            } else {
                planner.set_inline_timer_attention("".into());
                planner.set_inline_timer_triggered_at("".into());
            }
        }
        let Some(window) = self.window.upgrade() else {
            return;
        };
        if surface != TimerAttentionSurface::Card {
            window.set_pulse_active(false);
            if window.window().is_visible() {
                let _ = window.hide();
            }
            return;
        }
        let Some(item) = front else {
            return;
        };
        if window.window().is_visible()
            && window.get_timer_title() == item.title
            && window.get_triggered_at() == item.triggered_at
        {
            return;
        }
        window.set_timer_title(item.title.into());
        window.set_triggered_at(item.triggered_at.into());
        let shown = show_auxiliary_window(
            window.window(),
            |_| {},
            |window| position_calendar_window(window, &self.owner),
            AuxiliaryFocusPolicy::Preserve,
        );
        window.set_pulse_active(shown);
    }
}

fn timer_triggered_label(timer: &PlannerTimer, settings: &AppSettings) -> String {
    let Some(triggered) = timer
        .attention_triggered_at_utc
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
    else {
        return String::new();
    };
    let zone = settings
        .clocks_settings
        .iter()
        .find(|clock| clock.is_main)
        .or_else(|| settings.clocks_settings.first())
        .and_then(|clock| clock.time_zone.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    triggered
        .with_timezone(&zone)
        .format("%d %b · %H:%M")
        .to_string()
}

fn schedule_planner_timer_refresh(
    alert_sound: AlertSound,
    timer: Rc<Timer>,
    model: Rc<VecModel<PlannerTimerData>>,
    settings: Rc<RefCell<AppSettings>>,
    presenter: TimerAttentionPresenter,
) {
    let next_timer = timer.clone();
    let next_model = model.clone();
    let next_settings = settings.clone();
    timer.start(TimerMode::SingleShot, Duration::from_secs(1), move || {
        let mut settings = next_settings.borrow_mut();
        let queue = &presenter.queue;
        let mut updated = settings.clone();
        let timers_before = updated.planner.timers.clone();
        let finished_ids = oziclock_app::timer_time::reconcile(&mut updated.planner, Utc::now());
        let timers_changed = timers_before != updated.planner.timers;
        if timers_changed && oziclock_storage::save(&updated).is_ok() {
            *settings = updated;
            for id in finished_ids {
                if let Some(timer) = settings.planner.timers.iter().find(|timer| timer.id == id) {
                    let title = timer_display_title(timer);
                    let triggered_at = timer_triggered_label(timer, &settings);
                    queue.borrow_mut().push_back(TimerAttentionItem {
                        timer_id: id.clone(),
                        title: title.clone(),
                        triggered_at: triggered_at.clone(),
                    });
                    if presenter.is_inline(&id.to_string()) {
                        delivery_feedback::deliver_in_window(
                            settings.alert_sound_duration_seconds,
                            &alert_sound,
                        );
                    } else {
                        delivery_feedback::deliver(
                            &title,
                            &triggered_at,
                            settings.alert_sound_duration_seconds,
                            &alert_sound,
                        );
                    }
                }
            }
        }
        if settings
            .planner
            .timers
            .iter()
            .any(|timer| timer.state == TimerState::Running || timer.state == TimerState::Finished)
        {
            // Keep the expanded component alive so its attention pulse retains
            // its phase while the countdown model refreshes once per second.
            for (index, row) in planner_timer_rows(&settings).into_iter().enumerate() {
                next_model.set_row_data(index, row);
            }
        }
        drop(settings);
        presenter.refresh();
        schedule_planner_timer_refresh(
            alert_sound.clone(),
            next_timer.clone(),
            next_model.clone(),
            next_settings.clone(),
            presenter.clone(),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expanded_view() -> TimerAttentionView<'static> {
        TimerAttentionView {
            visible: true,
            focused: true,
            timers_section: true,
            editor_open: false,
            expanded_timer: "break",
        }
    }

    #[test]
    fn tmr_12_matching_expanded_timer_is_inline_only_while_pending() {
        let view = expanded_view();
        assert_eq!(
            timer_attention_surface(Some("break"), &view),
            TimerAttentionSurface::Inline
        );
        assert_eq!(
            timer_attention_surface(None, &view),
            TimerAttentionSurface::Hidden
        );
        assert_eq!(
            timer_attention_surface(Some("other"), &view),
            TimerAttentionSurface::Card
        );
    }

    #[test]
    fn tmr_12_focus_hide_navigation_collapse_and_modal_use_card() {
        let view = expanded_view();
        for unavailable in [
            TimerAttentionView {
                visible: false,
                ..view
            },
            TimerAttentionView {
                focused: false,
                ..view
            },
            TimerAttentionView {
                timers_section: false,
                ..view
            },
            TimerAttentionView {
                expanded_timer: "",
                ..view
            },
            TimerAttentionView {
                editor_open: true,
                ..view
            },
        ] {
            assert_eq!(
                timer_attention_surface(Some("break"), &unavailable),
                TimerAttentionSurface::Card
            );
        }
    }

    #[test]
    fn tmr_12_fifo_front_controls_the_surface_without_skipping_other_timers() {
        let mut queue = VecDeque::from(["other", "break"]);
        let view = expanded_view();
        assert_eq!(
            timer_attention_surface(queue.front().copied(), &view),
            TimerAttentionSurface::Card
        );
        queue.pop_front();
        assert_eq!(
            timer_attention_surface(queue.front().copied(), &view),
            TimerAttentionSurface::Inline
        );
        queue.pop_front();
        assert_eq!(
            timer_attention_surface(queue.front().copied(), &view),
            TimerAttentionSurface::Hidden
        );
    }

    #[test]
    fn tmr_12_missing_windows_preserve_pending_attention_for_recovery() {
        let queue = Rc::new(RefCell::new(VecDeque::from([TimerAttentionItem {
            timer_id: PlannerId::new("break").unwrap(),
            title: "Break — 05:00".into(),
            triggered_at: "21 Oct · 10:05".into(),
        }])));
        let presenter = TimerAttentionPresenter {
            window: slint::Weak::default(),
            planner: slint::Weak::default(),
            owner: slint::Weak::default(),
            queue: queue.clone(),
        };
        presenter.refresh();
        presenter.refresh();
        assert_eq!(queue.borrow().len(), 1);
        assert_eq!(queue.borrow()[0].timer_id.to_string(), "break");
        assert!(!presenter.is_inline("break"));
    }

    #[test]
    fn tmr_12_dismiss_and_restart_retry_without_changing_state_after_failed_save() {
        for restart in [false, true] {
            let mut settings: AppSettings = serde_json::from_str(include_str!(
                "../../../../crates/oziclock-storage/assets/default_settings.json"
            ))
            .unwrap();
            let id = PlannerId::new("break").unwrap();
            settings.planner.timers = vec![PlannerTimer {
                id: id.clone(),
                title: "Break".into(),
                duration_seconds: 300,
                remaining_seconds: 0,
                state: TimerState::Finished,
                repeat: false,
                repeat_count: Some(0),
                repeats_remaining: Some(0),
                started_at_utc: None,
                attention_pending: true,
                attention_triggered_at_utc: Some("2026-10-21T10:05:00Z".into()),
            }];
            let original = settings.planner.timers.clone();
            let command = || {
                if restart {
                    PlannerCommand::RestartTimer {
                        id: id.clone(),
                        started_at_utc: "2026-10-21T10:06:00Z".into(),
                    }
                } else {
                    PlannerCommand::DismissTimer { id: id.clone() }
                }
            };
            assert!(!commit_timer_attention(&mut settings, command(), |_| false));
            assert_eq!(settings.planner.timers, original);
            assert!(commit_timer_attention(&mut settings, command(), |_| true));
            let timer = &settings.planner.timers[0];
            assert!(!timer.attention_pending);
            assert!(timer.attention_triggered_at_utc.is_none());
            if restart {
                assert_eq!(timer.state, TimerState::Running);
                assert_eq!(timer.remaining_seconds, 300);
                assert_eq!(
                    timer.started_at_utc.as_deref(),
                    Some("2026-10-21T10:06:00Z")
                );
            } else {
                assert_eq!(timer.state, TimerState::Idle);
                assert_eq!(timer.remaining_seconds, 0);
            }
        }
    }
}
