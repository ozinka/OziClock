use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use oziclock_app::planner::{PlannerCommand, execute_planner_command};
use oziclock_storage::{AppSettings, Stopwatch, StopwatchState};
use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel};

use super::{
    PlannerWindow, StopwatchLapData,
    planner_models::{format_stopwatch, stopwatch_lap_rows},
};

pub(super) fn wire_stopwatch_bindings(
    planner_window: &PlannerWindow,
    shared_settings: Rc<RefCell<AppSettings>>,
) {
    let stopwatch_started_at = Rc::new(Cell::new(None::<Instant>));
    if shared_settings
        .borrow()
        .planner
        .stopwatch
        .as_ref()
        .is_some_and(|stopwatch| stopwatch.state == StopwatchState::Running)
    {
        shared_settings
            .borrow_mut()
            .planner
            .stopwatch
            .as_mut()
            .expect("running stopwatch exists")
            .interrupt();
    }
    let stopwatch_lap_model = Rc::new(VecModel::from(stopwatch_lap_rows(
        &shared_settings.borrow(),
    )));
    planner_window.set_stopwatch_laps(ModelRc::from(stopwatch_lap_model.clone()));
    refresh_stopwatch_ui(
        planner_window,
        &shared_settings.borrow(),
        None,
        &stopwatch_lap_model,
    );

    let stopwatch_for_toggle = planner_window.as_weak();
    let settings_for_stopwatch_toggle = shared_settings.clone();
    let started_for_stopwatch_toggle = stopwatch_started_at.clone();
    let laps_for_stopwatch_toggle = stopwatch_lap_model.clone();
    planner_window.on_request_toggle_stopwatch(move || {
        let mut settings = settings_for_stopwatch_toggle.borrow_mut();
        if settings.planner.stopwatch.is_none() {
            settings.planner.stopwatch = Some(Stopwatch {
                state: StopwatchState::Idle,
                elapsed_seconds: 0,
                elapsed_milliseconds: 0,
                laps_seconds: vec![],
                laps_milliseconds: vec![],
            });
        }
        let elapsed_milliseconds =
            stopwatch_elapsed_milliseconds(&settings, started_for_stopwatch_toggle.get());
        let command = if settings
            .planner
            .stopwatch
            .as_ref()
            .is_some_and(|stopwatch| stopwatch.state == StopwatchState::Running)
        {
            started_for_stopwatch_toggle.set(None);
            PlannerCommand::PauseStopwatch {
                elapsed_milliseconds: elapsed_milliseconds as u64,
            }
        } else {
            started_for_stopwatch_toggle.set(Some(Instant::now()));
            PlannerCommand::StartStopwatch
        };
        if execute_planner_command(&mut settings.planner, command) {
            let _ = oziclock_storage::save(&settings);
            if let Some(planner) = stopwatch_for_toggle.upgrade() {
                refresh_stopwatch_ui(
                    &planner,
                    &settings,
                    started_for_stopwatch_toggle.get(),
                    &laps_for_stopwatch_toggle,
                );
            }
        }
    });

    let stopwatch_for_reset = planner_window.as_weak();
    let settings_for_stopwatch_reset = shared_settings.clone();
    let started_for_stopwatch_reset = stopwatch_started_at.clone();
    let laps_for_stopwatch_reset = stopwatch_lap_model.clone();
    planner_window.on_request_reset_stopwatch(move || {
        let mut settings = settings_for_stopwatch_reset.borrow_mut();
        started_for_stopwatch_reset.set(None);
        if execute_planner_command(&mut settings.planner, PlannerCommand::ResetStopwatch) {
            let _ = oziclock_storage::save(&settings);
            if let Some(planner) = stopwatch_for_reset.upgrade() {
                refresh_stopwatch_ui(&planner, &settings, None, &laps_for_stopwatch_reset);
            }
        }
    });

    let stopwatch_for_lap = planner_window.as_weak();
    let settings_for_stopwatch_lap = shared_settings.clone();
    let started_for_stopwatch_lap = stopwatch_started_at.clone();
    let laps_for_stopwatch_lap = stopwatch_lap_model.clone();
    planner_window.on_request_stopwatch_lap(move || {
        let mut settings = settings_for_stopwatch_lap.borrow_mut();
        let elapsed_milliseconds =
            stopwatch_elapsed_milliseconds(&settings, started_for_stopwatch_lap.get()) as u64;
        if execute_planner_command(
            &mut settings.planner,
            PlannerCommand::RecordStopwatchLap {
                elapsed_milliseconds,
            },
        ) {
            let _ = oziclock_storage::save(&settings);
            if let Some(planner) = stopwatch_for_lap.upgrade() {
                refresh_stopwatch_ui(
                    &planner,
                    &settings,
                    started_for_stopwatch_lap.get(),
                    &laps_for_stopwatch_lap,
                );
            }
        }
    });

    let stopwatch_for_undo = planner_window.as_weak();
    let settings_for_stopwatch_undo = shared_settings.clone();
    let started_for_stopwatch_undo = stopwatch_started_at.clone();
    let laps_for_stopwatch_undo = stopwatch_lap_model.clone();
    planner_window.on_request_undo_stopwatch_lap(move || {
        let mut settings = settings_for_stopwatch_undo.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(&mut updated.planner, PlannerCommand::UndoStopwatchLap)
            && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            if let Some(planner) = stopwatch_for_undo.upgrade() {
                refresh_stopwatch_ui(
                    &planner,
                    &settings,
                    started_for_stopwatch_undo.get(),
                    &laps_for_stopwatch_undo,
                );
            }
        }
    });

    let stopwatch_for_clear = planner_window.as_weak();
    let settings_for_stopwatch_clear = shared_settings.clone();
    let started_for_stopwatch_clear = stopwatch_started_at.clone();
    let laps_for_stopwatch_clear = stopwatch_lap_model.clone();
    planner_window.on_request_clear_stopwatch_laps(move || {
        let mut settings = settings_for_stopwatch_clear.borrow_mut();
        let mut updated = settings.clone();
        if execute_planner_command(&mut updated.planner, PlannerCommand::ClearStopwatchLaps)
            && oziclock_storage::save(&updated).is_ok()
        {
            *settings = updated;
            if let Some(planner) = stopwatch_for_clear.upgrade() {
                refresh_stopwatch_ui(
                    &planner,
                    &settings,
                    started_for_stopwatch_clear.get(),
                    &laps_for_stopwatch_clear,
                );
            }
        }
    });

    schedule_stopwatch_refresh(
        Rc::new(Timer::default()),
        planner_window.as_weak(),
        shared_settings,
        stopwatch_started_at,
        stopwatch_lap_model,
    );
}

fn stopwatch_elapsed_milliseconds(settings: &AppSettings, started_at: Option<Instant>) -> u128 {
    settings.planner.stopwatch.as_ref().map_or(0, |stopwatch| {
        u128::from(stopwatch.elapsed_seconds) * 1_000 + u128::from(stopwatch.elapsed_milliseconds)
    }) + started_at
        .map(|started_at| started_at.elapsed().as_millis())
        .unwrap_or(0)
}

fn refresh_stopwatch_ui(
    planner: &PlannerWindow,
    settings: &AppSettings,
    started_at: Option<Instant>,
    lap_model: &VecModel<StopwatchLapData>,
) {
    let (time, milliseconds) =
        format_stopwatch(stopwatch_elapsed_milliseconds(settings, started_at));
    planner.set_stopwatch_time(time.into());
    planner.set_stopwatch_milliseconds(milliseconds.into());
    planner.set_stopwatch_running(
        settings
            .planner
            .stopwatch
            .as_ref()
            .is_some_and(|stopwatch| stopwatch.state == StopwatchState::Running),
    );
    planner.set_stopwatch_has_data(
        stopwatch_elapsed_milliseconds(settings, started_at) > 0
            || !stopwatch_lap_rows(settings).is_empty(),
    );
    lap_model.set_vec(stopwatch_lap_rows(settings));
}

fn schedule_stopwatch_refresh(
    timer: Rc<Timer>,
    planner: slint::Weak<PlannerWindow>,
    settings: Rc<RefCell<AppSettings>>,
    started_at: Rc<Cell<Option<Instant>>>,
    lap_model: Rc<VecModel<StopwatchLapData>>,
) {
    let next_timer = timer.clone();
    let next_planner = planner.clone();
    let next_settings = settings.clone();
    let next_started_at = started_at.clone();
    let next_lap_model = lap_model.clone();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(16),
        move || {
            if let Some(planner) = next_planner.upgrade() {
                let settings = next_settings.borrow();
                if settings
                    .planner
                    .stopwatch
                    .as_ref()
                    .is_some_and(|stopwatch| stopwatch.state == StopwatchState::Running)
                {
                    refresh_stopwatch_ui(
                        &planner,
                        &settings,
                        next_started_at.get(),
                        &next_lap_model,
                    );
                }
            }
            schedule_stopwatch_refresh(
                next_timer.clone(),
                next_planner.clone(),
                next_settings.clone(),
                next_started_at.clone(),
                next_lap_model.clone(),
            );
        },
    );
}
