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
    apply_time_zone_filter, main_clock_index, move_clock_to, move_selected_clock,
    open_settings_window, persist_settings_window_size, select_clock, selected_time_zone_id,
    time_zone_options, update_settings_preview,
};
#[cfg(target_os = "windows")]
use tray::create_system_tray;
use window_drag::configure_main_window_drag;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use chrono::{DateTime, LocalResult, Offset, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
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
    let mut settings = oziclock_storage::load_or_initialize().map_err(|error| {
        slint::PlatformError::Other(format!("could not load OziClock settings: {error}"))
    })?;
    let clock_scale_percent = normalize_clock_scale_percent((settings.clock_scale * 100.0) as f32);
    settings.clock_scale = f64::from(clock_scale_percent / 100.0);
    settings.border_color =
        normalize_border_color(&settings.border_color).unwrap_or_else(|| "#000000".to_owned());
    settings.non_main_dimming = settings.non_main_dimming.clamp(0.0, 80.0);
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
    window.set_compact_progress(if settings.compact_mode { 1.0 } else { 0.0 });
    window.set_corner_radius(settings.corner_radius.clamp(0.0, 15.5) as f32);
    window.set_soft_clock_style(settings.soft_clock_style);
    window.set_border_color(parse_color(&settings.border_color));
    window.set_non_main_dimming(settings.non_main_dimming as f32);
    apply_clock_scale(&window, settings.clock_scale.clamp(0.8, 1.5) as f32);
    update_clock_tiles(&window, &settings.clocks_settings, settings.show_seconds);

    let settings_window = SettingsWindow::new()?;
    let settings_for_keyboard = settings_window.as_weak();
    settings_window
        .window()
        .on_winit_window_event(move |_, event| {
            if is_escape_key(event) || matches!(event, WindowEvent::CloseRequested) {
                if let Some(settings_window) = settings_for_keyboard.upgrade() {
                    settings_window.invoke_request_close();
                }
                return EventResult::PreventDefault;
            }
            if is_enter_key(event) {
                if let Some(settings_window) = settings_for_keyboard.upgrade() {
                    if settings_window.get_time_zone_search_focused()
                        || settings_window.get_time_zone_picker_focused()
                    {
                        return EventResult::Propagate;
                    }
                    settings_window.invoke_request_save();
                }
                return EventResult::PreventDefault;
            }
            EventResult::Propagate
        });
    let now = Utc::now();
    let time_zone_options = Rc::new(time_zone_options(now));
    apply_time_zone_filter(&settings_window, &time_zone_options, "");
    settings_window.set_show_seconds(settings.show_seconds);
    settings_window.set_top_most(settings.top_most);
    settings_window.set_show_in_task_bar(settings.show_in_task_bar);
    settings_window.set_compact_mode(settings.compact_mode);
    settings_window.set_show_rulers(settings.show_rulers);
    settings_window.set_clock_scale_percent(clock_scale_percent);
    settings_window.set_corner_radius(settings.corner_radius.clamp(0.0, 15.5) as f32);
    settings_window.set_soft_clock_style(settings.soft_clock_style);
    settings_window.set_border_color_value(settings.border_color.clone().into());
    settings_window.set_border_preview_color(parse_color(&settings.border_color));
    settings_window.set_non_main_dimming(settings.non_main_dimming as f32);
    settings_window.set_opacity_percent((settings.opacity.clamp(0.02, 1.0) * 100.0) as f32);
    update_settings_preview(&settings_window, &settings.clocks_settings);
    select_clock(&settings_window, &settings.clocks_settings, 0);
    settings_window.set_selected_section(0);
    initialize_ruler_content(&window, &settings);
    let shared_settings = Rc::new(RefCell::new(settings));
    let explored_time = Rc::new(RefCell::new(None::<DateTime<Utc>>));
    let clock_timer = Rc::new(Timer::default());
    let ruler_label_settings = shared_settings.clone();
    window.on_format_label(move |column_index, hour| {
        format_ruler_label(&ruler_label_settings.borrow(), column_index, hour).into()
    });
    let main_window_for_ruler = window.as_weak();
    let content_for_ruler = window.as_weak();
    let settings_for_ruler = shared_settings.clone();
    let explored_time_for_ruler = explored_time.clone();
    window.on_request_focus_progress(move |progress| {
        let time_step = (progress * 288.0).round();
        if let Some(content) = content_for_ruler.upgrade() {
            content.set_focus_progress(progress);
            content.set_time_step(time_step);
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
    let content_for_slider = window.as_weak();
    let main_window_for_slider = window.as_weak();
    let settings_for_slider = shared_settings.clone();
    let explored_time_for_slider = explored_time.clone();
    window.on_request_time_step(move |time_step| {
        if let Some(content) = content_for_slider.upgrade() {
            content.set_focus_progress((time_step / 288.0).clamp(0.0, 1.0));
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
    let content_for_focus = window.as_weak();
    window.on_request_focus_column(move |column_index| {
        if let Some(content) = content_for_focus.upgrade() {
            let maximum_index = content.get_rulers().row_count().saturating_sub(1) as i32;
            content.set_focused_column(column_index.clamp(0, maximum_index));
        }
    });
    if shared_settings.borrow().show_rulers {
        window.invoke_request_focus_progress(window.get_focus_progress());
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
    let main_window_for_settings_close = window.as_weak();
    settings_window.on_request_close(move || {
        if let Some(settings_window) = weak_settings_window.upgrade() {
            let mut settings = settings_for_close.borrow_mut();
            persist_settings_window_size(&settings_window, &mut settings);
            let _ = oziclock_storage::save(&settings);
            let _ = settings_window.hide();
        }
        set_main_window_modal(&main_window_for_settings_close, false);
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
            initialize_ruler_content(&main_window, &state);
            sync_main_window_size(&main_window);
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
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
                    initialize_ruler_content(&main_window, &state);
                    sync_main_window_size(&main_window);
                }
            }
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
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
                if let Some(main_window) = main_window.upgrade() {
                    initialize_ruler_content(&main_window, &settings);
                    if settings.show_rulers {
                        main_window.invoke_request_focus_progress(main_window.get_focus_progress());
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
        {
            let mut state = state.borrow_mut();
            state.compact_mode = compact_mode;
        }
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_compact_mode(compact_mode);
            animate_compact_mode(
                main_window.as_weak(),
                compact_mode,
                compact_animation_for_settings.clone(),
            );
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let explored_time_for_visibility = explored_time.clone();
    settings_window.on_request_set_show_rulers(move |show_rulers| {
        let settings = {
            let mut state = state.borrow_mut();
            state.show_rulers = show_rulers;
            state.clone()
        };
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_show_rulers(show_rulers);
            if show_rulers {
                initialize_ruler_content(&main_window, &settings);
                main_window.invoke_request_focus_progress(main_window.get_focus_progress());
            }
            sync_main_window_size(&main_window);
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
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
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
            if show_rulers {
                initialize_ruler_content(&main_window, &settings);
                main_window.invoke_request_focus_progress(main_window.get_focus_progress());
            }
            sync_main_window_size(&main_window);
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
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let editor = settings_window.as_weak();
    settings_window.on_request_set_clock_scale(move |clock_scale_percent| {
        let clock_scale_percent = normalize_clock_scale_percent(clock_scale_percent);
        let clock_scale = clock_scale_percent / 100.0;
        {
            let mut state = state.borrow_mut();
            state.clock_scale = f64::from(clock_scale);
        }
        if let Some(editor) = editor.upgrade() {
            editor.set_clock_scale_percent(clock_scale_percent);
        }
        if let Some(main_window) = main_window.upgrade() {
            apply_clock_scale(&main_window, clock_scale);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    settings_window.on_request_set_corner_radius(move |corner_radius| {
        let corner_radius = corner_radius.clamp(0.0, 15.5);
        state.borrow_mut().corner_radius = f64::from(corner_radius);
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_corner_radius(corner_radius);
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
    let editor = settings_window.as_weak();
    settings_window.on_request_set_border_color(move |border_color| {
        let Some(border_color) = normalize_border_color(border_color.as_str()) else {
            return;
        };
        state.borrow_mut().border_color = border_color.clone();
        let border_color = parse_color(&border_color);
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_border_color(border_color);
        }
        if let Some(editor) = editor.upgrade() {
            editor.set_border_preview_color(border_color);
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    let editor = settings_window.as_weak();
    settings_window.on_request_set_non_main_dimming(move |non_main_dimming| {
        let non_main_dimming = ((non_main_dimming / 5.0).round() * 5.0).clamp(0.0, 80.0);
        state.borrow_mut().non_main_dimming = f64::from(non_main_dimming);
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_non_main_dimming(non_main_dimming);
        }
        if let Some(editor) = editor.upgrade() {
            editor.set_non_main_dimming(non_main_dimming);
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
        let compact_mode = {
            let mut state = state.borrow_mut();
            state.compact_mode = !state.compact_mode;
            state.compact_mode
        };
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_compact_mode(compact_mode);
            animate_compact_mode(
                main_window.as_weak(),
                compact_mode,
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
    settings_window.on_request_select_time_zone(move |index| {
        if let Some(editor) = editor.upgrade()
            && let Some(time_zone) = selected_time_zone_id(&editor, index)
        {
            editor.set_editor_time_zone(time_zone.clone().into());
            editor.invoke_request_apply();
        }
    });
    let editor = settings_window.as_weak();
    let time_zones_for_filter = time_zone_options.clone();
    settings_window.on_request_filter_time_zones(move |query| {
        if let Some(editor) = editor.upgrade() {
            apply_time_zone_filter(&editor, &time_zones_for_filter, query.as_str());
        }
    });
    let state = shared_settings.clone();
    let saved = saved_settings.clone();
    let editor = settings_window.as_weak();
    let main_window_for_move_up = window.as_weak();
    settings_window.on_request_move_up(move || {
        if let Some(editor) = editor.upgrade() {
            let mut state = state.borrow_mut();
            move_selected_clock(&editor, &mut state.clocks_settings, -1);
            refresh_clock_order(&main_window_for_move_up, &state);
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window_for_move_down = window.as_weak();
    settings_window.on_request_move_down(move || {
        if let Some(editor) = editor.upgrade() {
            let mut state = state.borrow_mut();
            move_selected_clock(&editor, &mut state.clocks_settings, 1);
            refresh_clock_order(&main_window_for_move_down, &state);
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
    settings_window.on_request_drag_clock(move |_index, y| {
        if let Some(editor) = editor.upgrade() {
            let current = drag_move.get();
            let target = (y / 43.0).floor() as i32;
            if current != target && target >= 0 {
                let mut state = state.borrow_mut();
                move_clock_to(&editor, &mut state.clocks_settings, current, target);
                refresh_clock_order(&main_window_for_drag, &state);
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
            editor.set_picking_border_color(false);
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
    let pending_color_for_border_open = pending_color.clone();
    let hue_for_border_open = hue.clone();
    let saturation_for_border_open = saturation.clone();
    let value_for_border_open = value.clone();
    settings_window.on_request_open_border_color_picker(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_picking_border_color(true);
            let color = editor.get_border_color_value().to_string();
            let (selected_hue, selected_saturation, selected_value) = color_to_hsv(&color);
            *pending_color_for_border_open.borrow_mut() = color;
            hue_for_border_open.set(selected_hue);
            saturation_for_border_open.set(selected_saturation);
            value_for_border_open.set(selected_value);
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
            if editor.get_picking_border_color() {
                editor.set_border_preview_color(parse_color(&color));
                editor.set_border_color_value(color.clone().into());
                editor.invoke_request_set_border_color(color.into());
            } else {
                editor.set_editor_preview_color(parse_color(&color));
                editor.set_editor_color(color.into());
                editor.invoke_request_apply();
            }
            editor.set_color_picker_open(false);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_pick_color(move |color| {
        if let Some(editor) = editor.upgrade() {
            if editor.get_picking_border_color() {
                editor.set_border_color_value(color.clone());
                editor.set_border_preview_color(parse_color(&color));
                editor.invoke_request_set_border_color(color);
            } else {
                editor.set_editor_color(color);
                editor.set_editor_preview_color(parse_color(&editor.get_editor_color()));
                editor.invoke_request_apply();
            }
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
            let color = hsv_hex(hue_for_color.get(), s, v);
            let preview = hsv_color(hue_for_color.get(), s, v);
            if editor.get_picking_border_color() {
                editor.set_border_color_value(color.clone().into());
                editor.set_border_preview_color(preview);
                editor.invoke_request_set_border_color(color.into());
            } else {
                editor.set_editor_color(color.into());
                editor.set_editor_preview_color(preview);
                editor.invoke_request_apply();
            }
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
            let color = hsv_hex(h, saturation_for_hue.get(), value_for_hue.get());
            let preview = hsv_color(h, saturation_for_hue.get(), value_for_hue.get());
            if editor.get_picking_border_color() {
                editor.set_border_color_value(color.clone().into());
                editor.set_border_preview_color(preview);
                editor.invoke_request_set_border_color(color.into());
            } else {
                editor.set_editor_color(color.into());
                editor.set_editor_preview_color(preview);
                editor.invoke_request_apply();
            }
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    let main_window = window.as_weak();
    let main_window_for_save_modal = window.as_weak();
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
                set_main_window_modal(&main_window_for_save_modal, false);
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
    about_window.set_version(env!("CARGO_PKG_VERSION").into());
    let about_for_keyboard = about_window.as_weak();
    about_window
        .window()
        .on_winit_window_event(move |_, event| {
            if is_escape_key(event)
                || is_enter_key(event)
                || matches!(event, WindowEvent::CloseRequested)
                || matches!(event, WindowEvent::Focused(false))
            {
                if let Some(about_window) = about_for_keyboard.upgrade() {
                    about_window.invoke_request_close();
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
    let main_window_for_about_close = window.as_weak();
    about_window.on_request_close(move || {
        if let Some(about_window) = weak_about_window.upgrade() {
            let _ = about_window.hide();
        }
        set_main_window_modal(&main_window_for_about_close, false);
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

    configure_main_window_drag(&window);
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
            let _ = slint::quit_event_loop();
            return EventResult::PreventDefault;
        }
        EventResult::Propagate
    });

    window.show()?;
    set_main_window_taskbar_visibility(&window, shared_settings.borrow().show_in_task_bar);
    sync_main_window_size(&window);
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
    set_main_window_modal(main_window, true);
    let _ = about_window.show();
    hide_auxiliary_window_from_taskbar(about_window.window());
    position_auxiliary_window_near_clock(about_window.window(), main_window);
    focus_auxiliary_window(about_window.window());
}

fn set_main_window_modal(main_window: &slint::Weak<AppWindow>, modal_open: bool) {
    if let Some(main_window) = main_window.upgrade() {
        main_window.set_modal_open(modal_open);
    }
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

fn normalize_clock_scale_percent(clock_scale_percent: f32) -> f32 {
    ((clock_scale_percent / 5.0).round() * 5.0).clamp(80.0, 150.0)
}

fn normalize_border_color(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('#');
    (value.len() == 6 && value.chars().all(|character| character.is_ascii_hexdigit()))
        .then(|| format!("#{}", value.to_ascii_uppercase()))
}

fn initialize_ruler_content(window: &AppWindow, settings: &AppSettings) {
    window.set_label_hours(ModelRc::new(VecModel::from((0..=24).collect::<Vec<i32>>())));
    window.set_rulers(ModelRc::new(VecModel::from(
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
    window.set_focused_column(focused_column);
    window.set_focus_column_position(focused_column as f32);
    let initial_time_step = initial_ruler_time_step(settings);
    window.set_focus_progress(initial_time_step / 288.0);
    window.set_tick_indices(ModelRc::new(VecModel::from(
        (0..=144).collect::<Vec<i32>>(),
    )));
    window.set_time_step(initial_time_step);
    window.set_hour_labels(ModelRc::new(VecModel::from(slider_hour_labels(
        settings.clocks_settings.len(),
    ))));
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

fn animate_compact_mode(
    window: slint::Weak<AppWindow>,
    compact_mode: bool,
    generation: Rc<Cell<u64>>,
) {
    let revision = generation.get().wrapping_add(1);
    generation.set(revision);
    let start_progress = window
        .upgrade()
        .map(|window| window.get_compact_progress())
        .unwrap_or(if compact_mode { 1.0 } else { 0.0 });
    animate_compact_mode_frame(
        window,
        start_progress,
        if compact_mode { 1.0 } else { 0.0 },
        Instant::now(),
        revision,
        generation,
    );
}

fn animate_compact_mode_frame(
    window: slint::Weak<AppWindow>,
    start_progress: f32,
    target_progress: f32,
    start: Instant,
    revision: u64,
    generation: Rc<Cell<u64>>,
) {
    let progress = (start.elapsed().as_secs_f32() / 0.2).clamp(0.0, 1.0);
    let eased = progress * progress * (3.0 - 2.0 * progress);
    let compact_progress = start_progress + (target_progress - start_progress) * eased;
    if let Some(window) = window.upgrade() {
        window.set_compact_progress(compact_progress);
        set_main_window_height_for_compact_progress(&window, compact_progress);
    }
    if progress < 1.0 {
        Timer::single_shot(Duration::from_millis(16), move || {
            if generation.get() == revision {
                animate_compact_mode_frame(
                    window,
                    start_progress,
                    target_progress,
                    start,
                    revision,
                    generation,
                );
            }
        });
    }
}

fn set_main_window_height_for_compact_progress(window: &AppWindow, compact_progress: f32) {
    let _ = window.window().with_winit_window(|native| {
        let logical_clock_height = 62.0 - 31.0 * compact_progress;
        let logical_height =
            logical_clock_height + if window.get_show_rulers() { 532.0 } else { 0.0 };
        let physical_height =
            (logical_height * window.get_clock_scale() * native.scale_factor() as f32).round()
                as u32;
        let _ = native.request_inner_size(PhysicalSize::new(
            native.inner_size().width,
            physical_height,
        ));
    });
}

fn sync_main_window_size(window: &AppWindow) {
    let _ = window.window().with_winit_window(|native| {
        let system_scale = native.scale_factor() as f32;
        let clock_scale = window.get_clock_scale();
        let logical_width = 1.0 + 100.0 * window.get_clocks().row_count() as f32;
        let clock_height = if window.get_compact_mode() {
            31.0
        } else {
            62.0
        };
        let logical_height = clock_height + if window.get_show_rulers() { 532.0 } else { 0.0 };
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
            let clock_height = if owner.get_compact_mode() { 31.0 } else { 62.0 };
            let below_owner = owner_position.y
                + (clock_height * owner.get_clock_scale() * owner_native.scale_factor() as f32)
                    .round() as i32
                + 8;
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

fn refresh_clock_order(main_window: &slint::Weak<AppWindow>, settings: &AppSettings) {
    if let Some(main_window) = main_window.upgrade() {
        update_clock_tiles(
            &main_window,
            &settings.clocks_settings,
            settings.show_seconds,
        );
        initialize_ruler_content(&main_window, settings);
        sync_main_window_size(&main_window);
    }
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
            let clock_height = if owner.get_compact_mode() { 31.0 } else { 62.0 };
            let below = owner_position.y
                + (clock_height * owner.get_clock_scale() * scale_factor as f32).round() as i32;
            let above = owner_position.y - menu_size.height as i32;
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
