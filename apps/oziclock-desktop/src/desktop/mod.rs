slint::include_modules!();

mod clock_refresh;
mod colors;
mod settings_bindings;
#[cfg(target_os = "windows")]
mod tray;
mod window_drag;

use clock_refresh::schedule_clock_refresh;
use colors::{color_to_hsv, hsv_color, hsv_hex, parse_color};
use settings_bindings::{
    main_clock_index, move_clock_to, move_selected_clock, open_settings_window,
    persist_settings_window_size, select_clock, time_zone_display_name, time_zone_offset_seconds,
    update_settings_preview,
};
#[cfg(target_os = "windows")]
use tray::create_system_tray;
use window_drag::{configure_main_window_drag, position_attached_windows};

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use chrono::{DateTime, LocalResult, Offset, TimeZone, Timelike, Utc};
use chrono_tz::{TZ_VARIANTS, Tz};
use oziclock_app::{ClockCommand, execute_clock_command};
use oziclock_storage::{AppSettings, ClockSettings};
use slint::winit_030::EventResult;
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::dpi::{LogicalPosition, PhysicalPosition, PhysicalSize};
#[cfg(target_os = "windows")]
use slint::winit_030::winit::platform::windows::{MonitorHandleExtWindows, WindowExtWindows};
use slint::winit_030::winit::{
    event::{ElementState, WindowEvent},
    keyboard::{Key, NamedKey},
};
use slint::{Model, ModelRc, Timer, VecModel};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO};

pub(crate) fn run() -> Result<(), slint::PlatformError> {
    let settings = oziclock_storage::load_or_initialize().map_err(|error| {
        slint::PlatformError::Other(format!("could not load OziClock settings: {error}"))
    })?;
    let initial_main_window_position =
        LogicalPosition::new(settings.main_wnd_left, settings.main_wnd_top);
    let is_first_native_window = Rc::new(Cell::new(true));
    let first_native_window_for_hook = is_first_native_window.clone();
    slint::BackendSelector::new()
        .with_winit_window_attributes_hook(move |attributes| {
            if first_native_window_for_hook.replace(false) {
                attributes.with_position(initial_main_window_position)
            } else {
                attributes
            }
        })
        .select()?;
    let window = AppWindow::new()?;
    window.set_product_name(oziclock_app::application_name().into());
    window.set_top_most(settings.top_most);
    window.set_inactive_opacity(settings.opacity.clamp(0.02, 1.0) as f32);
    window.set_show_seconds(settings.show_seconds);
    window.set_show_rulers(settings.show_rulers);
    window.set_compact_mode(settings.compact_mode);
    window.set_corner_radius(settings.corner_radius.clamp(0.0, 15.5) as f32);
    window.set_soft_clock_style(settings.soft_clock_style);
    apply_clock_scale(&window, settings.clock_scale.clamp(0.8, 1.5) as f32);
    update_clock_tiles(&window, &settings.clocks_settings, settings.show_seconds);

    let settings_window = SettingsWindow::new()?;
    let settings_for_keyboard = settings_window.as_weak();
    settings_window
        .window()
        .on_winit_window_event(move |_, event| {
            if is_escape_key(event) {
                if let Some(settings_window) = settings_for_keyboard.upgrade() {
                    settings_window.invoke_request_close();
                }
                return EventResult::PreventDefault;
            }
            if is_enter_key(event) {
                if let Some(settings_window) = settings_for_keyboard.upgrade() {
                    settings_window.invoke_request_save();
                }
                return EventResult::PreventDefault;
            }
            EventResult::Propagate
        });
    let rulers_window = RulersWindow::new()?;
    let time_slider_window = TimeSliderWindow::new()?;
    let now = Utc::now();
    let mut time_zone_options: Vec<(String, String, i32)> = TZ_VARIANTS
        .iter()
        .map(|time_zone| {
            (
                time_zone.to_string(),
                time_zone_display_name(*time_zone, now),
                time_zone_offset_seconds(*time_zone, now),
            )
        })
        .collect();
    time_zone_options
        .sort_by(|left, right| left.2.cmp(&right.2).then_with(|| left.0.cmp(&right.0)));
    let time_zones: Rc<Vec<String>> = Rc::new(
        time_zone_options
            .iter()
            .map(|(name, _, _)| name.clone())
            .collect(),
    );
    settings_window.set_time_zones(ModelRc::new(VecModel::from(
        time_zone_options
            .iter()
            .map(|(_, display_name, _)| display_name.clone().into())
            .collect::<Vec<_>>(),
    )));
    settings_window.set_show_seconds(settings.show_seconds);
    settings_window.set_top_most(settings.top_most);
    settings_window.set_show_in_task_bar(settings.show_in_task_bar);
    settings_window.set_compact_mode(settings.compact_mode);
    settings_window.set_show_rulers(settings.show_rulers);
    settings_window.set_clock_scale_percent((settings.clock_scale.clamp(0.8, 1.5) * 100.0) as f32);
    settings_window.set_corner_radius(settings.corner_radius.clamp(0.0, 15.5) as f32);
    settings_window.set_soft_clock_style(settings.soft_clock_style);
    settings_window.set_opacity_percent((settings.opacity.clamp(0.02, 1.0) * 100.0) as f32);
    update_settings_preview(&settings_window, &settings.clocks_settings);
    select_clock(&settings_window, &settings.clocks_settings, 0);
    settings_window.set_selected_section(0);
    initialize_ruler_windows(&rulers_window, &time_slider_window, &settings);
    let shared_settings = Rc::new(RefCell::new(settings));
    let explored_time = Rc::new(RefCell::new(None::<DateTime<Utc>>));
    let clock_timer = Rc::new(Timer::default());
    let ruler_label_settings = shared_settings.clone();
    rulers_window.on_format_label(move |column_index, hour| {
        format_ruler_label(&ruler_label_settings.borrow(), column_index, hour).into()
    });
    let slider_for_ruler = time_slider_window.as_weak();
    let rulers_for_ruler = rulers_window.as_weak();
    let main_window_for_ruler = window.as_weak();
    let settings_for_ruler = shared_settings.clone();
    let explored_time_for_ruler = explored_time.clone();
    rulers_window.on_request_focus_progress(move |progress| {
        if let Some(rulers_window) = rulers_for_ruler.upgrade() {
            rulers_window.set_focus_progress(progress);
        }
        let time_step = (progress * 288.0).round();
        if let Some(time_slider_window) = slider_for_ruler.upgrade() {
            time_slider_window.set_time_step(time_step);
        }
        let selected_time = ruler_time_step_to_utc(&settings_for_ruler.borrow(), time_step);
        *explored_time_for_ruler.borrow_mut() = Some(selected_time);
        if let Some(main_window) = main_window_for_ruler.upgrade() {
            let settings = settings_for_ruler.borrow();
            update_clock_tiles_at(
                &main_window,
                &settings.clocks_settings,
                settings.show_seconds,
                selected_time,
            );
        }
    });
    let rulers_for_slider = rulers_window.as_weak();
    let main_window_for_slider = window.as_weak();
    let settings_for_slider = shared_settings.clone();
    let explored_time_for_slider = explored_time.clone();
    time_slider_window.on_request_time_step(move |time_step| {
        if let Some(rulers_window) = rulers_for_slider.upgrade() {
            rulers_window.set_focus_progress((time_step / 288.0).clamp(0.0, 1.0));
        }
        let selected_time = ruler_time_step_to_utc(&settings_for_slider.borrow(), time_step);
        *explored_time_for_slider.borrow_mut() = Some(selected_time);
        if let Some(main_window) = main_window_for_slider.upgrade() {
            let settings = settings_for_slider.borrow();
            update_clock_tiles_at(
                &main_window,
                &settings.clocks_settings,
                settings.show_seconds,
                selected_time,
            );
        }
    });
    let rulers_for_focus = rulers_window.as_weak();
    rulers_window.on_request_focus_column(move |column_index| {
        if let Some(rulers_window) = rulers_for_focus.upgrade() {
            let maximum_index = rulers_window.get_rulers().row_count().saturating_sub(1) as i32;
            rulers_window.set_focused_column(column_index.clamp(0, maximum_index));
        }
    });
    if shared_settings.borrow().show_rulers {
        rulers_window.invoke_request_focus_progress(rulers_window.get_focus_progress());
    }
    let saved_settings = Rc::new(RefCell::new(shared_settings.borrow().clone()));
    let weak_settings_window = settings_window.as_weak();
    let settings_for_open = shared_settings.clone();
    let main_window_for_settings = window.as_weak();
    window.on_request_open_settings(move || {
        if let Some(settings_window) = weak_settings_window.upgrade() {
            open_settings_window(
                &settings_window,
                &main_window_for_settings,
                &settings_for_open.borrow(),
            );
        }
    });
    let weak_settings_window = settings_window.as_weak();
    let settings_for_close = shared_settings.clone();
    settings_window.on_request_close(move || {
        if let Some(settings_window) = weak_settings_window.upgrade() {
            let mut settings = settings_for_close.borrow_mut();
            persist_settings_window_size(&settings_window, &mut settings);
            let _ = oziclock_storage::save(&settings);
            let _ = settings_window.hide();
        }
    });
    let drag_settings_window = settings_window.as_weak();
    settings_window.on_request_window_drag(move || {
        if let Some(settings_window) = drag_settings_window.upgrade() {
            let _ = settings_window
                .window()
                .with_winit_window(|window| window.drag_window());
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    settings_window.on_request_select(move |index| {
        if let Some(editor) = editor.upgrade() {
            select_clock(&editor, &state.borrow().clocks_settings, index);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_select_general(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_selected_section(0);
            editor.set_color_picker_open(false);
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
    let rulers_for_add = rulers_window.as_weak();
    let slider_for_add = time_slider_window.as_weak();
    settings_window.on_request_add(move || {
        let mut state = state.borrow_mut();
        execute_clock_command(
            &mut state.clocks_settings,
            ClockCommand::Add(ClockSettings {
                label: "UTC".into(),
                time_zone: "UTC".into(),
                color: "#FFFFFFFF".into(),
                is_main: false,
            }),
        );
        let index = state.clocks_settings.len() as i32 - 1;
        update_settings_preview(&editor.upgrade().unwrap(), &state.clocks_settings);
        select_clock(&editor.upgrade().unwrap(), &state.clocks_settings, index);
        if let Some(main_window) = main_window.upgrade() {
            update_clock_tiles(&main_window, &state.clocks_settings, state.show_seconds);
            sync_main_window_size(&main_window);
        }
        if let (Some(rulers_window), Some(time_slider_window)) =
            (rulers_for_add.upgrade(), slider_for_add.upgrade())
        {
            initialize_ruler_windows(&rulers_window, &time_slider_window, &state);
        }
        sync_attached_windows(
            &main_window,
            &rulers_for_add,
            &slider_for_add,
            state.show_rulers,
        );
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
    let rulers_for_remove = rulers_window.as_weak();
    let slider_for_remove = time_slider_window.as_weak();
    settings_window.on_request_remove(move || {
        if let Some(editor) = editor.upgrade() {
            let mut state = state.borrow_mut();
            let index = editor.get_selected_index();
            if index >= 0
                && execute_clock_command(
                    &mut state.clocks_settings,
                    ClockCommand::Remove {
                        index: index as usize,
                    },
                )
            {
                update_settings_preview(&editor, &state.clocks_settings);
                select_clock(&editor, &state.clocks_settings, 0);
                if let Some(main_window) = main_window.upgrade() {
                    update_clock_tiles(&main_window, &state.clocks_settings, state.show_seconds);
                    sync_main_window_size(&main_window);
                }
                if let (Some(rulers_window), Some(time_slider_window)) =
                    (rulers_for_remove.upgrade(), slider_for_remove.upgrade())
                {
                    initialize_ruler_windows(&rulers_window, &time_slider_window, &state);
                }
                sync_attached_windows(
                    &main_window,
                    &rulers_for_remove,
                    &slider_for_remove,
                    state.show_rulers,
                );
            }
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
    let rulers_for_apply = rulers_window.as_weak();
    let slider_for_apply = time_slider_window.as_weak();
    settings_window.on_request_apply(move || {
        if let Some(editor) = editor.upgrade() {
            let index = editor.get_selected_index();
            let selected_index = index.max(0) as usize;
            let make_main = editor.get_editor_is_main();
            let (settings, main_clock_changed) = {
                let mut state = state.borrow_mut();
                let previous_main_clock = main_clock_index(&state.clocks_settings);
                if let Some(clock) = state.clocks_settings.get_mut(selected_index) {
                    clock.label = editor.get_editor_label().to_string();
                    clock.time_zone = editor.get_editor_time_zone().to_string();
                    clock.color = editor.get_editor_color().to_string();
                }
                if selected_index < state.clocks_settings.len() {
                    if make_main {
                        execute_clock_command(
                            &mut state.clocks_settings,
                            ClockCommand::SetMain {
                                index: selected_index,
                            },
                        );
                    } else if previous_main_clock == selected_index {
                        editor.set_editor_is_main(true);
                    } else if let Some(clock) = state.clocks_settings.get_mut(selected_index) {
                        clock.is_main = false;
                    }
                }
                let main_clock_changed =
                    previous_main_clock != main_clock_index(&state.clocks_settings);
                (state.clone(), main_clock_changed)
            };
            if main_clock_changed {
                update_settings_preview(&editor, &settings.clocks_settings);
                if let (Some(rulers_window), Some(time_slider_window)) =
                    (rulers_for_apply.upgrade(), slider_for_apply.upgrade())
                {
                    initialize_ruler_windows(&rulers_window, &time_slider_window, &settings);
                    if settings.show_rulers {
                        rulers_window
                            .invoke_request_focus_progress(rulers_window.get_focus_progress());
                    }
                }
            }
            if let Some(main_window) = main_window.upgrade() {
                update_clock_tiles(
                    &main_window,
                    &settings.clocks_settings,
                    settings.show_seconds,
                );
            }
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let timer_for_seconds = clock_timer.clone();
    let explored_time_for_seconds = explored_time.clone();
    settings_window.on_request_set_show_seconds(move |show_seconds| {
        {
            let mut state = state.borrow_mut();
            state.show_seconds = show_seconds;
            if let Some(main_window) = main_window.upgrade() {
                main_window.set_show_seconds(show_seconds);
                update_clock_tiles(&main_window, &state.clocks_settings, show_seconds);
            }
        }
        schedule_clock_refresh(
            timer_for_seconds.clone(),
            main_window.clone(),
            state.clone(),
            explored_time_for_seconds.clone(),
        );
    });
    let compact_animation_generation = Rc::new(Cell::new(0_u64));
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let compact_animation_for_settings = compact_animation_generation.clone();
    settings_window.on_request_set_compact_mode(move |compact_mode| {
        let clock_scale = {
            let mut state = state.borrow_mut();
            state.compact_mode = compact_mode;
            state.clock_scale as f32
        };
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_compact_mode(compact_mode);
            animate_main_window_height(
                main_window.as_weak(),
                (if compact_mode { 31.0 } else { 62.0 }) * clock_scale,
                compact_animation_for_settings.clone(),
            );
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let rulers_for_visibility = rulers_window.as_weak();
    let slider_for_visibility = time_slider_window.as_weak();
    let explored_time_for_visibility = explored_time.clone();
    settings_window.on_request_set_show_rulers(move |show_rulers| {
        let settings = {
            let mut state = state.borrow_mut();
            state.show_rulers = show_rulers;
            state.clone()
        };
        if show_rulers
            && let (Some(rulers_window), Some(time_slider_window)) = (
                rulers_for_visibility.upgrade(),
                slider_for_visibility.upgrade(),
            )
        {
            initialize_ruler_windows(&rulers_window, &time_slider_window, &settings);
            rulers_window.invoke_request_focus_progress(rulers_window.get_focus_progress());
        }
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_show_rulers(show_rulers);
        }
        if !show_rulers {
            *explored_time_for_visibility.borrow_mut() = None;
            if let Some(main_window) = main_window.upgrade() {
                update_clock_tiles(
                    &main_window,
                    &settings.clocks_settings,
                    settings.show_seconds,
                );
            }
        }
        sync_attached_windows(
            &main_window,
            &rulers_for_visibility,
            &slider_for_visibility,
            show_rulers,
        );
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
    let rulers_for_toggle = rulers_window.as_weak();
    let slider_for_toggle = time_slider_window.as_weak();
    let explored_time_for_toggle = explored_time.clone();
    window.on_request_toggle_rulers(move || {
        let settings = {
            let mut state = state.borrow_mut();
            state.show_rulers = !state.show_rulers;
            state.clone()
        };
        let show_rulers = settings.show_rulers;
        if let Some(editor) = editor.upgrade() {
            editor.set_show_rulers(show_rulers);
        }
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_show_rulers(show_rulers);
        }
        if show_rulers
            && let (Some(rulers_window), Some(time_slider_window)) =
                (rulers_for_toggle.upgrade(), slider_for_toggle.upgrade())
        {
            initialize_ruler_windows(&rulers_window, &time_slider_window, &settings);
            rulers_window.invoke_request_focus_progress(rulers_window.get_focus_progress());
        }
        if !show_rulers {
            *explored_time_for_toggle.borrow_mut() = None;
            if let Some(main_window) = main_window.upgrade() {
                update_clock_tiles(
                    &main_window,
                    &settings.clocks_settings,
                    settings.show_seconds,
                );
            }
        }
        sync_attached_windows(
            &main_window,
            &rulers_for_toggle,
            &slider_for_toggle,
            show_rulers,
        );
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let rulers_for_scale = rulers_window.as_weak();
    let slider_for_scale = time_slider_window.as_weak();
    settings_window.on_request_set_clock_scale(move |clock_scale_percent| {
        let clock_scale = (clock_scale_percent / 100.0).clamp(0.8, 1.5);
        let show_rulers = {
            let mut state = state.borrow_mut();
            state.clock_scale = f64::from(clock_scale);
            state.show_rulers
        };
        if let Some(main_window) = main_window.upgrade() {
            apply_clock_scale(&main_window, clock_scale);
        }
        if let Some(rulers_window) = rulers_for_scale.upgrade() {
            rulers_window.set_clock_scale(clock_scale);
        }
        if let Some(time_slider_window) = slider_for_scale.upgrade() {
            time_slider_window.set_clock_scale(clock_scale);
        }
        sync_attached_windows(
            &main_window,
            &rulers_for_scale,
            &slider_for_scale,
            show_rulers,
        );
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let slider_for_radius = time_slider_window.as_weak();
    settings_window.on_request_set_corner_radius(move |corner_radius| {
        let corner_radius = corner_radius.clamp(0.0, 15.5);
        state.borrow_mut().corner_radius = f64::from(corner_radius);
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_corner_radius(corner_radius);
        }
        if let Some(time_slider_window) = slider_for_radius.upgrade() {
            time_slider_window.set_corner_radius(corner_radius);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    settings_window.on_request_set_soft_clock_style(move |soft_clock_style| {
        state.borrow_mut().soft_clock_style = soft_clock_style;
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_soft_clock_style(soft_clock_style);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    settings_window.on_request_set_top_most(move |top_most| {
        state.borrow_mut().top_most = top_most;
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_top_most(top_most);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let editor = settings_window.as_weak();
    window.on_request_toggle_top_most(move || {
        let top_most = {
            let mut state = state.borrow_mut();
            state.top_most = !state.top_most;
            state.top_most
        };
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_top_most(top_most);
        }
        if let Some(editor) = editor.upgrade() {
            editor.set_top_most(top_most);
        }
        let _ = oziclock_storage::save(&state.borrow());
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    settings_window.on_request_set_show_in_task_bar(move |show_in_task_bar| {
        state.borrow_mut().show_in_task_bar = show_in_task_bar;
        if let Some(main_window) = main_window.upgrade() {
            set_main_window_taskbar_visibility(&main_window, show_in_task_bar);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let editor = settings_window.as_weak();
    let compact_animation_for_middle_click = compact_animation_generation.clone();
    window.on_request_toggle_compact(move || {
        let (compact_mode, clock_scale) = {
            let mut state = state.borrow_mut();
            state.compact_mode = !state.compact_mode;
            (state.compact_mode, state.clock_scale as f32)
        };
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_compact_mode(compact_mode);
            animate_main_window_height(
                main_window.as_weak(),
                (if compact_mode { 31.0 } else { 62.0 }) * clock_scale,
                compact_animation_for_middle_click.clone(),
            );
        }
        if let Some(editor) = editor.upgrade() {
            editor.set_compact_mode(compact_mode);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    settings_window.on_request_set_opacity(move |opacity_percent| {
        let opacity = (opacity_percent / 100.0).clamp(0.02, 1.0);
        state.borrow_mut().opacity = f64::from(opacity);
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_inactive_opacity(opacity);
        }
    });
    let editor = settings_window.as_weak();
    let time_zones_for_select = time_zones.clone();
    settings_window.on_request_select_time_zone(move |index| {
        if let Some(time_zone) = time_zones_for_select.get(index.max(0) as usize)
            && let Some(editor) = editor.upgrade()
        {
            editor.set_editor_time_zone(time_zone.clone().into());
            editor.invoke_request_apply();
        }
    });
    let state = shared_settings.clone();
    let saved = saved_settings.clone();
    let editor = settings_window.as_weak();
    let main_window_for_move_up = window.as_weak();
    let rulers_for_move_up = rulers_window.as_weak();
    let slider_for_move_up = time_slider_window.as_weak();
    settings_window.on_request_move_up(move || {
        if let Some(editor) = editor.upgrade() {
            let mut state = state.borrow_mut();
            move_selected_clock(&editor, &mut state.clocks_settings, -1);
            refresh_clock_order(
                &main_window_for_move_up,
                &rulers_for_move_up,
                &slider_for_move_up,
                &state,
            );
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window_for_move_down = window.as_weak();
    let rulers_for_move_down = rulers_window.as_weak();
    let slider_for_move_down = time_slider_window.as_weak();
    settings_window.on_request_move_down(move || {
        if let Some(editor) = editor.upgrade() {
            let mut state = state.borrow_mut();
            move_selected_clock(&editor, &mut state.clocks_settings, 1);
            refresh_clock_order(
                &main_window_for_move_down,
                &rulers_for_move_down,
                &slider_for_move_down,
                &state,
            );
        }
    });
    let drag_index = Rc::new(Cell::new(-1));
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let drag_start = drag_index.clone();
    settings_window.on_request_list_press(move |x, y| {
        let index = (y / 43.0).floor() as i32;
        let clock_count = state.borrow().clocks_settings.len() as i32;
        if index < 0 || index >= clock_count {
            return;
        }
        if let Some(editor) = editor.upgrade() {
            if x >= 202.0 {
                drag_start.set(index);
                editor.set_dragging_index(index);
            } else {
                select_clock(&editor, &state.borrow().clocks_settings, index);
            }
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let drag_move = drag_index.clone();
    let main_window_for_drag = window.as_weak();
    let rulers_for_drag = rulers_window.as_weak();
    let slider_for_drag = time_slider_window.as_weak();
    settings_window.on_request_drag_clock(move |_index, y| {
        if let Some(editor) = editor.upgrade() {
            let current = drag_move.get();
            let target = (y / 43.0).floor() as i32;
            if current != target && target >= 0 {
                let mut state = state.borrow_mut();
                move_clock_to(&editor, &mut state.clocks_settings, current, target);
                refresh_clock_order(
                    &main_window_for_drag,
                    &rulers_for_drag,
                    &slider_for_drag,
                    &state,
                );
                drag_move.set(target);
                editor.set_dragging_index(target);
            }
        }
    });
    let editor = settings_window.as_weak();
    let drag_end = drag_index.clone();
    settings_window.on_request_drag_end(move || {
        drag_end.set(-1);
        if let Some(editor) = editor.upgrade() {
            editor.set_dragging_index(-1);
        }
    });
    let hue = Rc::new(Cell::new(220.0_f32));
    let saturation = Rc::new(Cell::new(70.0_f32));
    let value = Rc::new(Cell::new(90.0_f32));
    let pending_color = Rc::new(RefCell::new(String::new()));
    let editor = settings_window.as_weak();
    let pending_color_for_open = pending_color.clone();
    let hue_for_open = hue.clone();
    let saturation_for_open = saturation.clone();
    let value_for_open = value.clone();
    settings_window.on_request_open_color_picker(move || {
        if let Some(editor) = editor.upgrade() {
            let color = editor.get_editor_color().to_string();
            let (selected_hue, selected_saturation, selected_value) = color_to_hsv(&color);
            *pending_color_for_open.borrow_mut() = color;
            hue_for_open.set(selected_hue);
            saturation_for_open.set(selected_saturation);
            value_for_open.set(selected_value);
            editor.set_picker_hue(selected_hue);
            editor.set_picker_saturation(selected_saturation);
            editor.set_picker_value(selected_value);
            editor.set_picker_hue_color(hsv_color(selected_hue, 100.0, 100.0));
            editor.set_color_picker_open(true);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_color_confirm(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_color_picker_open(false);
        }
    });
    let editor = settings_window.as_weak();
    let pending_color_for_cancel = pending_color.clone();
    settings_window.on_request_color_cancel(move || {
        if let Some(editor) = editor.upgrade() {
            let color = pending_color_for_cancel.borrow().clone();
            editor.set_editor_preview_color(parse_color(&color));
            editor.set_editor_color(color.into());
            editor.invoke_request_apply();
            editor.set_color_picker_open(false);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_pick_color(move |color| {
        if let Some(editor) = editor.upgrade() {
            editor.set_editor_color(color);
            editor.set_editor_preview_color(parse_color(&editor.get_editor_color()));
            editor.invoke_request_apply();
            editor.set_color_picker_open(false);
        }
    });
    let editor = settings_window.as_weak();
    let hue_for_color = hue.clone();
    let saturation_for_color = saturation.clone();
    let value_for_color = value.clone();
    settings_window.on_request_picker_color(move |x, y| {
        if let Some(editor) = editor.upgrade() {
            let s = (x / 426.0 * 100.0).clamp(0.0, 100.0);
            let v = (100.0 - y / 180.0 * 100.0).clamp(0.0, 100.0);
            saturation_for_color.set(s);
            value_for_color.set(v);
            editor.set_picker_saturation(s);
            editor.set_picker_value(v);
            editor.set_editor_color(hsv_hex(hue_for_color.get(), s, v).into());
            editor.set_editor_preview_color(hsv_color(hue_for_color.get(), s, v));
            editor.invoke_request_apply();
        }
    });
    let editor = settings_window.as_weak();
    let hue_for_hue = hue.clone();
    let saturation_for_hue = saturation.clone();
    let value_for_hue = value.clone();
    settings_window.on_request_picker_hue(move |x| {
        if let Some(editor) = editor.upgrade() {
            let h = (x / 426.0 * 360.0).clamp(0.0, 360.0);
            hue_for_hue.set(h);
            editor.set_picker_hue(h);
            editor.set_picker_hue_color(hsv_color(h, 100.0, 100.0));
            editor
                .set_editor_color(hsv_hex(h, saturation_for_hue.get(), value_for_hue.get()).into());
            editor.set_editor_preview_color(hsv_color(
                h,
                saturation_for_hue.get(),
                value_for_hue.get(),
            ));
            editor.invoke_request_apply();
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
    settings_window.on_request_save(move || {
        if let Some(editor) = editor.upgrade() {
            let mut state = state.borrow_mut();
            let index = editor.get_selected_index();
            if index < 0 || editor.get_editor_time_zone().parse::<Tz>().is_err() {
                editor.set_status_message(
                    "Enter a valid IANA time zone, for example Europe/Kyiv.".into(),
                );
                return;
            }
            let item = &mut state.clocks_settings[index as usize];
            item.label = editor.get_editor_label().to_string();
            item.time_zone = editor.get_editor_time_zone().to_string();
            item.color = editor.get_editor_color().to_string();
            let set_as_main = editor.get_editor_is_main();
            item.is_main = set_as_main;
            if set_as_main {
                execute_clock_command(
                    &mut state.clocks_settings,
                    ClockCommand::SetMain {
                        index: index as usize,
                    },
                );
            }
            if let Some(main_window) = main_window.upgrade() {
                persist_main_window_position(&main_window, &mut state);
            }
            persist_settings_window_size(&editor, &mut state);
            match oziclock_storage::save(&state) {
                Ok(()) => {
                    *saved.borrow_mut() = state.clone();
                    editor.set_status_message("Saved beside the executable.".into())
                }
                Err(error) => editor.set_status_message(format!("Save failed: {error}").into()),
            };
            update_settings_preview(&editor, &state.clocks_settings);
            if let Some(main_window) = main_window.upgrade() {
                update_clock_tiles(&main_window, &state.clocks_settings, state.show_seconds);
            }
            if editor.get_status_message() == "Saved beside the executable." {
                let _ = editor.hide();
            }
        }
    });

    let context_menu = ContextMenuWindow::new()?;
    let menu_for_dismiss = context_menu.as_weak();
    context_menu
        .window()
        .on_winit_window_event(move |_, event| {
            if matches!(event, WindowEvent::Focused(false))
                && let Some(context_menu) = menu_for_dismiss.upgrade()
            {
                let _ = context_menu.hide();
            }
            if is_escape_key(event) {
                if let Some(context_menu) = menu_for_dismiss.upgrade() {
                    let _ = context_menu.hide();
                }
                return EventResult::PreventDefault;
            }
            EventResult::Propagate
        });
    let weak_context_menu = context_menu.as_weak();
    let weak_window_for_menu = window.as_weak();
    window.on_request_open_menu(move || {
        if let (Some(context_menu), Some(window)) =
            (weak_context_menu.upgrade(), weak_window_for_menu.upgrade())
        {
            show_context_menu(&context_menu, &window);
        }
    });

    let about_window = AboutWindow::new()?;
    let about_for_keyboard = about_window.as_weak();
    about_window
        .window()
        .on_winit_window_event(move |_, event| {
            if is_escape_key(event) || is_enter_key(event) {
                if let Some(about_window) = about_for_keyboard.upgrade() {
                    let _ = about_window.hide();
                }
                return EventResult::PreventDefault;
            }
            EventResult::Propagate
        });
    let weak_about_window = about_window.as_weak();
    let main_window_for_about = window.as_weak();
    window.on_request_open_about(move || {
        if let Some(about_window) = weak_about_window.upgrade() {
            open_about_window(&about_window, &main_window_for_about);
        }
    });
    let weak_about_window = about_window.as_weak();
    about_window.on_request_close(move || {
        if let Some(about_window) = weak_about_window.upgrade() {
            let _ = about_window.hide();
        }
    });
    about_window.on_request_open_project_url(|| {
        let _ = webbrowser::open("https://github.com/ozinka/OziClock");
    });

    let weak_context_menu = context_menu.as_weak();
    let weak_settings_window = settings_window.as_weak();
    let main_window_for_context_settings = window.as_weak();
    let settings_for_context_open = shared_settings.clone();
    context_menu.on_request_open_settings(move || {
        if let Some(context_menu) = weak_context_menu.upgrade() {
            let _ = context_menu.hide();
        }
        if let Some(settings_window) = weak_settings_window.upgrade() {
            open_settings_window(
                &settings_window,
                &main_window_for_context_settings,
                &settings_for_context_open.borrow(),
            );
        }
    });
    let weak_context_menu = context_menu.as_weak();
    let weak_about_window = about_window.as_weak();
    let main_window_for_context_about = window.as_weak();
    context_menu.on_request_open_about(move || {
        if let Some(context_menu) = weak_context_menu.upgrade() {
            let _ = context_menu.hide();
        }
        if let Some(about_window) = weak_about_window.upgrade() {
            open_about_window(&about_window, &main_window_for_context_about);
        }
    });
    let exit_window = window.as_weak();
    let exit_settings_window = settings_window.as_weak();
    let exit_settings = shared_settings.clone();
    context_menu.on_request_exit(move || {
        save_state_before_exit(&exit_window, &exit_settings_window, &exit_settings);
        let _ = slint::quit_event_loop();
    });
    let exit_window = window.as_weak();
    let exit_settings_window = settings_window.as_weak();
    let exit_settings = shared_settings.clone();
    window.on_request_tray_exit(move || {
        save_state_before_exit(&exit_window, &exit_settings_window, &exit_settings);
        let _ = slint::quit_event_loop();
    });

    configure_main_window_drag(
        &window,
        rulers_window.as_weak(),
        time_slider_window.as_weak(),
    );
    let state = shared_settings.clone();
    let weak_window = window.as_weak();
    let explored_time_for_refresh = explored_time.clone();
    window.on_request_refresh_time(move || {
        if explored_time_for_refresh.borrow().is_none()
            && let Some(window) = weak_window.upgrade()
        {
            let state = state.borrow();
            update_clock_tiles(&window, &state.clocks_settings, state.show_seconds);
        }
    });
    schedule_clock_refresh(
        clock_timer.clone(),
        window.as_weak(),
        shared_settings.clone(),
        explored_time.clone(),
    );

    let main_window_for_attached_layout = window.as_weak();
    let rulers_for_attached_layout = rulers_window.as_weak();
    let slider_for_attached_layout = time_slider_window.as_weak();
    let settings_for_attached_layout = shared_settings.clone();
    let settings_window_for_shutdown = settings_window.as_weak();
    let context_menu_for_shutdown = context_menu.as_weak();
    let about_window_for_shutdown = about_window.as_weak();
    window.window().on_winit_window_event(move |_, event| {
        if matches!(event, WindowEvent::CloseRequested) {
            save_state_before_exit(
                &main_window_for_attached_layout,
                &settings_window_for_shutdown,
                &settings_for_attached_layout,
            );
            if let Some(settings_window) = settings_window_for_shutdown.upgrade() {
                let _ = settings_window.hide();
            }
            if let Some(context_menu) = context_menu_for_shutdown.upgrade() {
                let _ = context_menu.hide();
            }
            if let Some(about_window) = about_window_for_shutdown.upgrade() {
                let _ = about_window.hide();
            }
            if let Some(rulers_window) = rulers_for_attached_layout.upgrade() {
                let _ = rulers_window.hide();
            }
            if let Some(time_slider_window) = slider_for_attached_layout.upgrade() {
                let _ = time_slider_window.hide();
            }
            let _ = slint::quit_event_loop();
            return EventResult::PreventDefault;
        }
        if settings_for_attached_layout.borrow().show_rulers
            && matches!(
                event,
                slint::winit_030::winit::event::WindowEvent::Moved(_)
                    | slint::winit_030::winit::event::WindowEvent::Resized(_)
            )
        {
            position_attached_windows(
                &main_window_for_attached_layout,
                &rulers_for_attached_layout,
                &slider_for_attached_layout,
            );
        }
        EventResult::Propagate
    });

    window.show()?;
    set_main_window_taskbar_visibility(&window, shared_settings.borrow().show_in_task_bar);
    sync_attached_windows(
        &window.as_weak(),
        &rulers_window.as_weak(),
        &time_slider_window.as_weak(),
        shared_settings.borrow().show_rulers,
    );
    #[cfg(target_os = "windows")]
    let _system_tray = create_system_tray(window.as_weak(), shared_settings.borrow().top_most)?;
    window.run()
}

fn save_state_before_exit(
    window: &slint::Weak<AppWindow>,
    settings_window: &slint::Weak<SettingsWindow>,
    settings: &Rc<RefCell<AppSettings>>,
) {
    if let Some(window) = window.upgrade() {
        persist_main_window_position(&window, &mut settings.borrow_mut());
    }
    if let Some(settings_window) = settings_window.upgrade() {
        persist_settings_window_size(&settings_window, &mut settings.borrow_mut());
    }
    let _ = oziclock_storage::save(&settings.borrow());
}

fn open_about_window(about_window: &AboutWindow, main_window: &slint::Weak<AppWindow>) {
    let _ = about_window.show();
    hide_auxiliary_window_from_taskbar(about_window.window());
    position_auxiliary_window_near_clock(about_window.window(), main_window);
    focus_auxiliary_window(about_window.window());
}

fn animate_main_window_height(
    window: slint::Weak<AppWindow>,
    target_height: f32,
    generation: Rc<Cell<u64>>,
) {
    let revision = generation.get().wrapping_add(1);
    generation.set(revision);
    let start_height = window
        .upgrade()
        .and_then(|window| main_window_height(&window))
        .unwrap_or(target_height);
    animate_main_window_height_frame(
        window,
        start_height,
        target_height,
        Instant::now(),
        revision,
        generation,
    );
}

fn animate_main_window_height_frame(
    window: slint::Weak<AppWindow>,
    start_height: f32,
    target_height: f32,
    start: Instant,
    revision: u64,
    generation: Rc<Cell<u64>>,
) {
    let elapsed = start.elapsed().as_secs_f32();
    let progress = (elapsed / 0.2).clamp(0.0, 1.0);
    let eased = progress * progress * (3.0 - 2.0 * progress);
    if let Some(window) = window.upgrade() {
        set_main_window_height(
            &window,
            start_height + (target_height - start_height) * eased,
        );
    }
    if progress < 1.0 {
        Timer::single_shot(Duration::from_millis(16), move || {
            if generation.get() == revision {
                animate_main_window_height_frame(
                    window,
                    start_height,
                    target_height,
                    start,
                    revision,
                    generation,
                );
            }
        });
    }
}

fn main_window_height(window: &AppWindow) -> Option<f32> {
    window.window().with_winit_window(|native| {
        native.inner_size().height as f32 / native.scale_factor() as f32
    })
}

fn set_main_window_height(window: &AppWindow, height: f32) {
    let _ = window.window().with_winit_window(|native| {
        let width = native.inner_size().width;
        let physical_height = (height * native.scale_factor() as f32).round() as u32;
        let _ = native.request_inner_size(PhysicalSize::new(width, physical_height));
    });
}

fn persist_main_window_position(window: &AppWindow, settings: &mut AppSettings) {
    let _ = window.window().with_winit_window(|native| {
        if let Ok(position) = native.outer_position() {
            let scale_factor = native.scale_factor();
            settings.main_wnd_left = position.x as f64 / scale_factor;
            settings.main_wnd_top = position.y as f64 / scale_factor;
        }
    });
}

fn set_main_window_taskbar_visibility(window: &AppWindow, show_in_task_bar: bool) {
    #[cfg(target_os = "windows")]
    let _ = window
        .window()
        .with_winit_window(|native| native.set_skip_taskbar(!show_in_task_bar));

    #[cfg(not(target_os = "windows"))]
    let _ = (window, show_in_task_bar);
}

fn apply_clock_scale(window: &AppWindow, clock_scale: f32) {
    window.set_clock_scale(clock_scale);
    sync_main_window_size(window);
}

fn initialize_ruler_windows(
    rulers_window: &RulersWindow,
    time_slider_window: &TimeSliderWindow,
    settings: &AppSettings,
) {
    rulers_window.set_label_hours(ModelRc::new(VecModel::from((0..=24).collect::<Vec<i32>>())));
    rulers_window.set_rulers(ModelRc::new(VecModel::from(
        settings
            .clocks_settings
            .iter()
            .map(|clock| RulerColumnData {
                accent: parse_color(&clock.color),
            })
            .collect::<Vec<_>>(),
    )));
    let focused_column = settings
        .clocks_settings
        .iter()
        .position(|clock| clock.is_main)
        .unwrap_or(0) as i32;
    rulers_window.set_focused_column(focused_column);
    rulers_window.set_focus_column_position(focused_column as f32);
    let initial_time_step = initial_ruler_time_step(settings);
    rulers_window.set_focus_progress(initial_time_step / 288.0);
    rulers_window.set_clock_scale(settings.clock_scale.clamp(0.8, 1.5) as f32);
    rulers_window.set_tick_indices(ModelRc::new(VecModel::from(
        (0..=144).collect::<Vec<i32>>(),
    )));
    time_slider_window.set_clock_count(settings.clocks_settings.len() as i32);
    time_slider_window.set_time_step(initial_time_step);
    time_slider_window.set_hour_labels(ModelRc::new(VecModel::from(slider_hour_labels(
        settings.clocks_settings.len(),
    ))));
    time_slider_window.set_clock_scale(settings.clock_scale.clamp(0.8, 1.5) as f32);
    time_slider_window.set_corner_radius(settings.corner_radius.clamp(0.0, 15.5) as f32);
}

fn initial_ruler_time_step(settings: &AppSettings) -> f32 {
    let main_zone = settings
        .clocks_settings
        .iter()
        .find(|clock| clock.is_main)
        .or_else(|| settings.clocks_settings.first())
        .and_then(|clock| clock.time_zone.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    let local_now = Utc::now().with_timezone(&main_zone);
    let rounded_hour = (local_now.hour() + u32::from(local_now.minute() >= 30)) % 24;
    (rounded_hour * 12) as f32
}

fn slider_hour_labels(clock_count: usize) -> Vec<i32> {
    match clock_count {
        0 | 1 => vec![0, 12, 24],
        2 => vec![0, 6, 12, 18, 24],
        3 => vec![0, 3, 6, 9, 12, 15, 18, 21, 24],
        _ => (0..=24).step_by(2).collect(),
    }
}

fn format_ruler_label(settings: &AppSettings, column_index: i32, hour: i32) -> String {
    let Some(column) = settings.clocks_settings.get(column_index.max(0) as usize) else {
        return String::new();
    };
    let main_index = settings
        .clocks_settings
        .iter()
        .position(|clock| clock.is_main)
        .unwrap_or(0);
    if column_index as usize == main_index && hour == 24 {
        return "24".into();
    }
    let main_zone = settings.clocks_settings[main_index]
        .time_zone
        .parse::<Tz>()
        .ok();
    let column_zone = column.time_zone.parse::<Tz>().ok();
    let now = Utc::now();
    let offset_hours = match (main_zone, column_zone) {
        (Some(main_zone), Some(column_zone)) => {
            (now.with_timezone(&column_zone)
                .offset()
                .fix()
                .local_minus_utc()
                - now
                    .with_timezone(&main_zone)
                    .offset()
                    .fix()
                    .local_minus_utc()) as f64
                / 3600.0
        }
        _ => 0.0,
    };
    let raw = (hour as f64 + offset_hours).rem_euclid(24.0);
    let whole_hours = raw.floor() as i32;
    let minutes = ((raw - f64::from(whole_hours)) * 60.0).round() as i32;
    if minutes == 0 {
        whole_hours.to_string()
    } else {
        format!("{whole_hours}:{minutes:02}")
    }
}

fn sync_attached_windows(
    main_window: &slint::Weak<AppWindow>,
    rulers_window: &slint::Weak<RulersWindow>,
    time_slider_window: &slint::Weak<TimeSliderWindow>,
    show_rulers: bool,
) {
    let (Some(main_window), Some(rulers_window), Some(time_slider_window)) = (
        main_window.upgrade(),
        rulers_window.upgrade(),
        time_slider_window.upgrade(),
    ) else {
        return;
    };
    if !show_rulers {
        let _ = rulers_window.hide();
        let _ = time_slider_window.hide();
        return;
    }
    let _ = main_window.window().with_winit_window(|main_native| {
        let Ok(main_position) = main_native.outer_position() else {
            return;
        };
        let system_scale = main_native.scale_factor() as f32;
        let main_size = main_native.inner_size();
        let width = (100.0 * main_window.get_clocks().row_count() as f32 + 1.0)
            * main_window.get_clock_scale()
            * system_scale;
        let ruler_height = 463.0 * main_window.get_clock_scale() * system_scale;
        let slider_height = 69.0 * main_window.get_clock_scale() * system_scale;
        let _ = rulers_window.window().with_winit_window(|ruler_native| {
            let _ = ruler_native.request_inner_size(PhysicalSize::new(
                width.round() as u32,
                ruler_height.round() as u32,
            ));
            ruler_native.set_outer_position(PhysicalPosition::new(
                main_position.x,
                main_position.y + main_size.height as i32,
            ));
        });
        let _ = time_slider_window
            .window()
            .with_winit_window(|slider_native| {
                let _ = slider_native.request_inner_size(PhysicalSize::new(
                    width.round() as u32,
                    slider_height.round() as u32,
                ));
                slider_native.set_outer_position(PhysicalPosition::new(
                    main_position.x,
                    main_position.y + main_size.height as i32 + ruler_height.round() as i32,
                ));
            });
    });
    hide_auxiliary_window_from_taskbar(rulers_window.window());
    hide_auxiliary_window_from_taskbar(time_slider_window.window());
    let _ = rulers_window.show();
    let _ = time_slider_window.show();
}

fn sync_main_window_size(window: &AppWindow) {
    let _ = window.window().with_winit_window(|native| {
        let system_scale = native.scale_factor() as f32;
        let clock_scale = window.get_clock_scale();
        let logical_width = 1.0 + 100.0 * window.get_clocks().row_count() as f32;
        let logical_height = if window.get_compact_mode() {
            31.0
        } else {
            62.0
        };
        let _ = native.request_inner_size(PhysicalSize::new(
            (logical_width * clock_scale * system_scale).round() as u32,
            (logical_height * clock_scale * system_scale).round() as u32,
        ));
    });
}

fn position_auxiliary_window_near_clock(window: &slint::Window, owner: &slint::Weak<AppWindow>) {
    let Some(owner) = owner.upgrade() else {
        return;
    };

    let _ = owner.window().with_winit_window(|owner_native| {
        let Ok(owner_position) = owner_native.outer_position() else {
            return;
        };
        let Some(work_area) = monitor_work_area(owner_native) else {
            return;
        };
        let _ = window.with_winit_window(|settings_native| {
            let owner_size = owner_native.outer_size();
            let settings_size = settings_native.outer_size();
            let maximum_left = (work_area.right - settings_size.width as i32).max(work_area.left);
            let maximum_top = (work_area.bottom - settings_size.height as i32).max(work_area.top);
            let preferred_left =
                owner_position.x + (owner_size.width as i32 - settings_size.width as i32) / 2;
            let below_owner = owner_position.y + owner_size.height as i32 + 8;
            let above_owner = owner_position.y - settings_size.height as i32 - 8;
            let preferred_top = if below_owner <= maximum_top {
                below_owner
            } else {
                above_owner
            };
            settings_native.set_outer_position(PhysicalPosition::new(
                preferred_left.clamp(work_area.left, maximum_left),
                preferred_top.clamp(work_area.top, maximum_top),
            ));
        });
    });
}

struct WorkArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(target_os = "windows")]
fn monitor_work_area(window: &slint::winit_030::winit::window::Window) -> Option<WorkArea> {
    let monitor = window.current_monitor()?;
    monitor_work_area_for_handle(monitor.hmonitor() as _)
}

#[cfg(target_os = "windows")]
fn monitor_work_area_for_handle(monitor: *mut std::ffi::c_void) -> Option<WorkArea> {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let success = unsafe { GetMonitorInfoW(monitor, &mut info) };
    if success == 0 {
        return None;
    }
    Some(WorkArea {
        left: info.rcWork.left,
        top: info.rcWork.top,
        right: info.rcWork.right,
        bottom: info.rcWork.bottom,
    })
}

#[cfg(not(target_os = "windows"))]
fn monitor_work_area(window: &slint::winit_030::winit::window::Window) -> Option<WorkArea> {
    let monitor = window.current_monitor()?;
    let position = monitor.position();
    let size = monitor.size();
    Some(WorkArea {
        left: position.x,
        top: position.y,
        right: position.x + size.width as i32,
        bottom: position.y + size.height as i32,
    })
}

fn update_clock_tiles(window: &AppWindow, settings: &[ClockSettings], show_seconds: bool) {
    update_clock_tiles_at(window, settings, show_seconds, Utc::now());
}

fn update_clock_tiles_at(
    window: &AppWindow,
    settings: &[ClockSettings],
    show_seconds: bool,
    now: DateTime<Utc>,
) {
    let clocks: Vec<ClockTileData> = settings
        .iter()
        .map(|settings| to_clock_tile(settings, now, show_seconds))
        .collect();

    window.set_clocks(ModelRc::new(VecModel::from(clocks)));
}

fn ruler_time_step_to_utc(settings: &AppSettings, time_step: f32) -> DateTime<Utc> {
    let main_zone = settings
        .clocks_settings
        .iter()
        .find(|clock| clock.is_main)
        .or_else(|| settings.clocks_settings.first())
        .and_then(|clock| clock.time_zone.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    let local_now = Utc::now().with_timezone(&main_zone);
    let selected_minutes = (time_step.round() as u32).clamp(0, 288) * 5;
    let selected_local =
        local_now
            .date_naive()
            .and_hms_opt(selected_minutes / 60, selected_minutes % 60, 0);
    selected_local
        .and_then(
            |selected_local| match main_zone.from_local_datetime(&selected_local) {
                LocalResult::Single(value) => Some(value.with_timezone(&Utc)),
                LocalResult::Ambiguous(value, _) => Some(value.with_timezone(&Utc)),
                LocalResult::None => None,
            },
        )
        .unwrap_or_else(Utc::now)
}

fn refresh_clock_order(
    main_window: &slint::Weak<AppWindow>,
    rulers_window: &slint::Weak<RulersWindow>,
    time_slider_window: &slint::Weak<TimeSliderWindow>,
    settings: &AppSettings,
) {
    if let Some(main_window) = main_window.upgrade() {
        update_clock_tiles(
            &main_window,
            &settings.clocks_settings,
            settings.show_seconds,
        );
        sync_main_window_size(&main_window);
    }
    if let (Some(rulers_window), Some(time_slider_window)) =
        (rulers_window.upgrade(), time_slider_window.upgrade())
    {
        initialize_ruler_windows(&rulers_window, &time_slider_window, settings);
    }
    sync_attached_windows(
        main_window,
        rulers_window,
        time_slider_window,
        settings.show_rulers,
    );
}

fn show_context_menu(context_menu: &ContextMenuWindow, owner: &AppWindow) {
    let _ = context_menu.show();
    hide_auxiliary_window_from_taskbar(context_menu.window());
    let requested_x = owner.get_menu_x();
    let _ = owner.window().with_winit_window(|winit_owner| {
        let owner_position = winit_owner.outer_position().unwrap_or_default();
        let scale_factor = winit_owner.scale_factor();
        let Some(work_area) = monitor_work_area(winit_owner) else {
            return;
        };
        let _ = context_menu.window().with_winit_window(|menu| {
            let menu_size = menu.outer_size();
            let requested_left = owner_position.x + (requested_x * scale_factor as f32) as i32;
            let maximum_left = work_area.right - menu_size.width as i32;
            let left = requested_left.clamp(work_area.left, maximum_left);
            let below = owner_position.y + winit_owner.outer_size().height as i32 + 4;
            let above = owner_position.y - menu_size.height as i32 - 4;
            let maximum_top = work_area.bottom - menu_size.height as i32;
            let top = if below <= maximum_top {
                below
            } else {
                above.clamp(work_area.top, maximum_top)
            };
            menu.set_outer_position(PhysicalPosition::new(left, top));
        });
    });
    focus_auxiliary_window(context_menu.window());
}

fn focus_auxiliary_window(window: &slint::Window) {
    let _ = window.with_winit_window(|window| window.focus_window());
}

fn is_escape_key(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::KeyboardInput { event, .. }
            if event.state == ElementState::Pressed
                && matches!(event.logical_key, Key::Named(NamedKey::Escape))
    )
}

fn is_enter_key(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::KeyboardInput { event, .. }
            if event.state == ElementState::Pressed
                && matches!(event.logical_key, Key::Named(NamedKey::Enter))
    )
}

#[cfg(target_os = "windows")]
fn hide_auxiliary_window_from_taskbar(window: &slint::Window) {
    let _ = window.with_winit_window(|window| window.set_skip_taskbar(true));
}

#[cfg(not(target_os = "windows"))]
fn hide_auxiliary_window_from_taskbar(_window: &slint::Window) {}

fn to_clock_tile(
    settings: &ClockSettings,
    now: DateTime<Utc>,
    show_seconds: bool,
) -> ClockTileData {
    let timezone = settings.time_zone.parse::<Tz>().unwrap_or(chrono_tz::UTC);
    let local_time = now.with_timezone(&timezone);
    ClockTileData {
        label: settings.label.clone().into(),
        month: local_time.format("%m/").to_string().into(),
        day: local_time.format("%d").to_string().into(),
        hour: local_time.format("%H").to_string().into(),
        minute: local_time.format("%M").to_string().into(),
        second: if show_seconds {
            local_time.format("%S").to_string().into()
        } else {
            "".into()
        },
        accent: parse_color(&settings.color),
        main_zone: settings.is_main,
    }
}
