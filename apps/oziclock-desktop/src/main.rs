#![windows_subsystem = "windows"]

slint::include_modules!();

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Offset, Utc};
use chrono_tz::{TZ_VARIANTS, Tz};
use oziclock_storage::{AppSettings, ClockSettings};
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::dpi::{PhysicalPosition, PhysicalSize};
#[cfg(target_os = "windows")]
use slint::winit_030::winit::platform::windows::{MonitorHandleExtWindows, WindowExtWindows};
use slint::{Color, Model, ModelRc, Timer, TimerMode, VecModel};
#[cfg(target_os = "windows")]
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::POINT,
    Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
};

fn main() -> Result<(), slint::PlatformError> {
    let settings = oziclock_storage::load_or_initialize().map_err(|error| {
        slint::PlatformError::Other(format!("could not load OziClock settings: {error}"))
    })?;
    let window = AppWindow::new()?;
    window.set_product_name(oziclock_app::application_name().into());
    window.set_top_most(settings.top_most);
    window.set_inactive_opacity(settings.opacity.clamp(0.02, 1.0) as f32);
    window.set_show_seconds(settings.show_seconds);
    window.set_compact_mode(settings.compact_mode);
    apply_clock_scale(&window, settings.clock_scale.clamp(0.8, 1.5) as f32);
    update_clock_tiles(&window, &settings.clocks_settings, settings.show_seconds);

    let settings_window = SettingsWindow::new()?;
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
    settings_window.set_clock_scale_percent((settings.clock_scale.clamp(0.8, 1.5) * 100.0) as f32);
    settings_window.set_opacity_percent((settings.opacity.clamp(0.02, 1.0) * 100.0) as f32);
    update_settings_preview(&settings_window, &settings.clocks_settings);
    select_clock(&settings_window, &settings.clocks_settings, 0);
    settings_window.set_selected_section(0);
    let shared_settings = Rc::new(RefCell::new(settings));
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
    settings_window.on_request_close(move || {
        if let Some(settings_window) = weak_settings_window.upgrade() {
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
    settings_window.on_request_add(move || {
        let mut state = state.borrow_mut();
        state.clocks_settings.push(ClockSettings {
            label: "UTC".into(),
            time_zone: "UTC".into(),
            color: "#FFFFFFFF".into(),
            is_main: false,
        });
        let index = state.clocks_settings.len() as i32 - 1;
        update_settings_preview(&editor.upgrade().unwrap(), &state.clocks_settings);
        select_clock(&editor.upgrade().unwrap(), &state.clocks_settings, index);
        if let Some(main_window) = main_window.upgrade() {
            update_clock_tiles(&main_window, &state.clocks_settings, state.show_seconds);
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
            if state.clocks_settings.len() > 1 && index >= 0 {
                state.clocks_settings.remove(index as usize);
                update_settings_preview(&editor, &state.clocks_settings);
                select_clock(&editor, &state.clocks_settings, 0);
                if let Some(main_window) = main_window.upgrade() {
                    update_clock_tiles(&main_window, &state.clocks_settings, state.show_seconds);
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
            if let Some(clock) = state
                .borrow_mut()
                .clocks_settings
                .get_mut(index.max(0) as usize)
            {
                clock.label = editor.get_editor_label().to_string();
                clock.time_zone = editor.get_editor_time_zone().to_string();
                clock.color = editor.get_editor_color().to_string();
                clock.is_main = editor.get_editor_is_main();
            }
            if let Some(main_window) = main_window.upgrade() {
                let state = state.borrow();
                update_clock_tiles(&main_window, &state.clocks_settings, state.show_seconds);
            }
        }
    });
    let state = shared_settings.clone();
    let main_window = window.as_weak();
    settings_window.on_request_set_show_seconds(move |show_seconds| {
        let mut state = state.borrow_mut();
        state.show_seconds = show_seconds;
        if let Some(main_window) = main_window.upgrade() {
            main_window.set_show_seconds(show_seconds);
            update_clock_tiles(&main_window, &state.clocks_settings, show_seconds);
        }
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
    settings_window.on_request_set_clock_scale(move |clock_scale_percent| {
        let clock_scale = (clock_scale_percent / 100.0).clamp(0.8, 1.5);
        state.borrow_mut().clock_scale = f64::from(clock_scale);
        if let Some(main_window) = main_window.upgrade() {
            apply_clock_scale(&main_window, clock_scale);
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
    settings_window.on_request_move_up(move || {
        if let Some(editor) = editor.upgrade() {
            move_selected_clock(&editor, &mut state.borrow_mut().clocks_settings, -1);
        }
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    settings_window.on_request_move_down(move || {
        if let Some(editor) = editor.upgrade() {
            move_selected_clock(&editor, &mut state.borrow_mut().clocks_settings, 1);
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
    settings_window.on_request_drag_clock(move |_index, y| {
        if let Some(editor) = editor.upgrade() {
            let current = drag_move.get();
            let target = (y / 43.0).floor() as i32;
            move_clock_to(
                &editor,
                &mut state.borrow_mut().clocks_settings,
                current,
                target,
            );
            if current != target && target >= 0 {
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
    let pending_color = Rc::new(RefCell::new(String::new()));
    let editor = settings_window.as_weak();
    let pending_color_for_open = pending_color.clone();
    settings_window.on_request_open_color_picker(move || {
        if let Some(editor) = editor.upgrade() {
            *pending_color_for_open.borrow_mut() = editor.get_editor_color().to_string();
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
    let hue = Rc::new(Cell::new(220.0_f32));
    let saturation = Rc::new(Cell::new(70.0_f32));
    let value = Rc::new(Cell::new(90.0_f32));
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
            item.is_main = editor.get_editor_is_main();
            if item.is_main {
                for (other_index, other) in state.clocks_settings.iter_mut().enumerate() {
                    if other_index != index as usize {
                        other.is_main = false;
                    }
                }
            }
            if let Some(main_window) = main_window.upgrade() {
                persist_main_window_position(&main_window, &mut state);
            }
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
    let weak_about_window = about_window.as_weak();
    window.on_request_open_about(move || {
        if let Some(about_window) = weak_about_window.upgrade() {
            let _ = about_window.show();
            hide_auxiliary_window_from_taskbar(about_window.window());
        }
    });
    let weak_about_window = about_window.as_weak();
    about_window.on_request_close(move || {
        if let Some(about_window) = weak_about_window.upgrade() {
            let _ = about_window.hide();
        }
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
    context_menu.on_request_open_about(move || {
        if let Some(context_menu) = weak_context_menu.upgrade() {
            let _ = context_menu.hide();
        }
        if let Some(about_window) = weak_about_window.upgrade() {
            let _ = about_window.show();
            hide_auxiliary_window_from_taskbar(about_window.window());
        }
    });
    let exit_window = window.as_weak();
    let exit_settings_window = settings_window.as_weak();
    let exit_settings = shared_settings.clone();
    context_menu.on_request_exit(move || {
        save_state_before_exit(&exit_window, &exit_settings_window, &exit_settings);
        let _ = slint::quit_event_loop();
    });

    let drag_window = window.as_weak();
    window.on_request_window_drag(move || {
        if let Some(window) = drag_window.upgrade() {
            let _ = window
                .window()
                .with_winit_window(|window| window.drag_window());
        }
    });
    let state = shared_settings.clone();
    let weak_window = window.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        if let Some(window) = weak_window.upgrade() {
            let state = state.borrow();
            update_clock_tiles(&window, &state.clocks_settings, state.show_seconds);
        }
    });

    window.show()?;
    set_main_window_taskbar_visibility(&window, shared_settings.borrow().show_in_task_bar);
    let saved_position_settings = shared_settings.borrow().clone();
    let weak_window_for_restore = window.as_weak();
    Timer::single_shot(Duration::from_millis(10), move || {
        if let Some(window) = weak_window_for_restore.upgrade() {
            restore_main_window_position(&window, &saved_position_settings);
        }
    });
    #[cfg(target_os = "windows")]
    let _system_tray = create_system_tray(
        window.as_weak(),
        settings_window.as_weak(),
        shared_settings.clone(),
    )?;
    window.run()
}

#[cfg(target_os = "windows")]
struct SystemTray {
    _icon: TrayIcon,
    _event_timer: Timer,
}

#[cfg(target_os = "windows")]
fn create_system_tray(
    main_window: slint::Weak<AppWindow>,
    settings_window: slint::Weak<SettingsWindow>,
    settings: Rc<RefCell<AppSettings>>,
) -> Result<SystemTray, slint::PlatformError> {
    let menu = Menu::new();
    let show_hide = MenuItem::with_id("show-hide", "Show/Hide", true, None);
    let open_settings = MenuItem::with_id("settings", "Settings", true, None);
    let always_on_top = CheckMenuItem::with_id(
        "always-on-top",
        "Always on top",
        true,
        settings.borrow().top_most,
        None,
    );
    let exit = MenuItem::with_id("exit", "Exit", true, None);
    menu.append_items(&[
        &show_hide,
        &open_settings,
        &always_on_top,
        &PredefinedMenuItem::separator(),
        &exit,
    ])
    .map_err(|error| slint::PlatformError::Other(format!("could not create tray menu: {error}")))?;
    let icon = Icon::from_resource_name("IDI_APP_ICON", Some((32, 32))).map_err(|error| {
        slint::PlatformError::Other(format!("could not load tray icon resource: {error}"))
    })?;
    let icon = TrayIconBuilder::new()
        .with_tooltip("OziClock")
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .build()
        .map_err(|error| {
            slint::PlatformError::Other(format!("could not create system tray icon: {error}"))
        })?;

    let visible = Rc::new(Cell::new(true));
    let event_timer = Timer::default();
    let visible_for_events = visible.clone();
    event_timer.start(TimerMode::Repeated, Duration::from_millis(100), move || {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id().0.as_str() {
                "show-hide" => {
                    if let Some(main_window) = main_window.upgrade() {
                        if visible_for_events.get() {
                            let _ = main_window.hide();
                            visible_for_events.set(false);
                        } else {
                            let _ = main_window.show();
                            visible_for_events.set(true);
                        }
                    }
                }
                "settings" => {
                    if let Some(settings_window) = settings_window.upgrade() {
                        open_settings_window(&settings_window, &main_window, &settings.borrow());
                    }
                }
                "always-on-top" => {
                    let top_most = {
                        let mut settings = settings.borrow_mut();
                        settings.top_most = !settings.top_most;
                        settings.top_most
                    };
                    always_on_top.set_checked(top_most);
                    if let Some(main_window) = main_window.upgrade() {
                        main_window.set_top_most(top_most);
                    }
                    if let Some(settings_window) = settings_window.upgrade() {
                        settings_window.set_top_most(top_most);
                    }
                    let _ = oziclock_storage::save(&settings.borrow());
                }
                "exit" => {
                    save_state_before_exit(&main_window, &settings_window, &settings);
                    let _ = slint::quit_event_loop();
                }
                _ => {}
            }
        }
    });

    Ok(SystemTray {
        _icon: icon,
        _event_timer: event_timer,
    })
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

fn open_settings_window(
    settings_window: &SettingsWindow,
    main_window: &slint::Weak<AppWindow>,
    settings: &AppSettings,
) {
    restore_settings_window_size(settings_window, settings);
    let _ = settings_window.show();
    hide_auxiliary_window_from_taskbar(settings_window.window());
    position_settings_near_clock(settings_window, main_window);
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

fn restore_main_window_position(window: &AppWindow, settings: &AppSettings) {
    let _ = window.window().with_winit_window(|native| {
        let scale_factor = native.scale_factor();
        let saved_position = PhysicalPosition::new(
            (settings.main_wnd_left * scale_factor).round() as i32,
            (settings.main_wnd_top * scale_factor).round() as i32,
        );
        let work_area =
            work_area_for_saved_position(saved_position).or_else(|| monitor_work_area(native));
        let Some(work_area) = work_area else {
            return;
        };
        native.set_outer_position(clamp_window_position(
            saved_position,
            native.outer_size(),
            work_area,
        ));
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

fn restore_settings_window_size(window: &SettingsWindow, settings: &AppSettings) {
    let width = settings.settings_window_width.max(760.0);
    let height = settings.settings_window_height.max(510.0);
    window.set_saved_window_width(width as f32);
    window.set_saved_window_height(height as f32);
    let _ = window.window().with_winit_window(|native| {
        let scale_factor = native.scale_factor();
        let _ = native.request_inner_size(PhysicalSize::new(
            (width * scale_factor).round() as u32,
            (height * scale_factor).round() as u32,
        ));
    });
}

fn persist_settings_window_size(window: &SettingsWindow, settings: &mut AppSettings) {
    let _ = window.window().with_winit_window(|native| {
        let scale_factor = native.scale_factor();
        let size = native.inner_size();
        settings.settings_window_width = size.width as f64 / scale_factor;
        settings.settings_window_height = size.height as f64 / scale_factor;
        window.set_saved_window_width(settings.settings_window_width as f32);
        window.set_saved_window_height(settings.settings_window_height as f32);
    });
}

fn clamp_window_position(
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    work_area: WorkArea,
) -> PhysicalPosition<i32> {
    let maximum_left = (work_area.right - size.width as i32).max(work_area.left);
    let maximum_top = (work_area.bottom - size.height as i32).max(work_area.top);
    PhysicalPosition::new(
        position.x.clamp(work_area.left, maximum_left),
        position.y.clamp(work_area.top, maximum_top),
    )
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

fn position_settings_near_clock(settings: &SettingsWindow, owner: &slint::Weak<AppWindow>) {
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
        let _ = settings.window().with_winit_window(|settings_native| {
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

#[cfg(target_os = "windows")]
fn work_area_for_saved_position(position: PhysicalPosition<i32>) -> Option<WorkArea> {
    let monitor = unsafe {
        MonitorFromPoint(
            POINT {
                x: position.x,
                y: position.y,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    };
    monitor_work_area_for_handle(monitor)
}

#[cfg(not(target_os = "windows"))]
fn work_area_for_saved_position(_position: PhysicalPosition<i32>) -> Option<WorkArea> {
    None
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
    let now = Utc::now();
    let clocks: Vec<ClockTileData> = settings
        .iter()
        .map(|settings| to_clock_tile(settings, now, show_seconds))
        .collect();

    window.set_clocks(ModelRc::new(VecModel::from(clocks)));
}

fn update_settings_preview(window: &SettingsWindow, settings: &[ClockSettings]) {
    let clocks: Vec<ClockListItem> = settings
        .iter()
        .map(|settings| ClockListItem {
            label: settings.label.clone().into(),
            time_zone: settings.time_zone.clone().into(),
            accent: parse_color(&settings.color),
            is_main: settings.is_main,
        })
        .collect();

    window.set_clocks(ModelRc::new(VecModel::from(clocks)));
}

fn select_clock(window: &SettingsWindow, settings: &[ClockSettings], index: i32) {
    if let Some(clock) = settings.get(index.max(0) as usize) {
        window.set_selected_section(1);
        window.set_selected_index(index.max(0));
        window.set_editor_label(clock.label.clone().into());
        window.set_editor_time_zone(clock.time_zone.clone().into());
        window.set_selected_time_zone_index(time_zone_index(&clock.time_zone));
        window.set_editor_color(clock.color.clone().into());
        window.set_editor_preview_color(parse_color(&clock.color));
        window.set_editor_is_main(clock.is_main);
        window.set_status_message("".into());
    }
}

fn time_zone_index(time_zone: &str) -> i32 {
    let now = Utc::now();
    let mut time_zones: Vec<(String, i32)> = TZ_VARIANTS
        .iter()
        .map(|candidate| {
            (
                candidate.to_string(),
                time_zone_offset_seconds(*candidate, now),
            )
        })
        .collect();
    time_zones.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
    time_zones
        .iter()
        .position(|(candidate, _)| candidate == time_zone)
        .map(|index| index as i32)
        .unwrap_or(-1)
}

fn time_zone_display_name(time_zone: Tz, now: DateTime<Utc>) -> String {
    let offset_seconds = time_zone_offset_seconds(time_zone, now);
    let sign = if offset_seconds < 0 { '-' } else { '+' };
    let total_minutes = offset_seconds.unsigned_abs() / 60;
    format!(
        "(UTC{sign}{:02}:{:02}) {time_zone}",
        total_minutes / 60,
        total_minutes % 60
    )
}

fn time_zone_offset_seconds(time_zone: Tz, now: DateTime<Utc>) -> i32 {
    now.with_timezone(&time_zone)
        .offset()
        .fix()
        .local_minus_utc()
}

fn move_selected_clock(window: &SettingsWindow, settings: &mut [ClockSettings], direction: i32) {
    let from = window.get_selected_index();
    let to = from + direction;
    if from < 0 || to < 0 || to >= settings.len() as i32 {
        return;
    }
    settings.swap(from as usize, to as usize);
    update_settings_preview(window, settings);
    select_clock(window, settings, to);
}

fn move_clock_to(window: &SettingsWindow, settings: &mut [ClockSettings], from: i32, to: i32) {
    if from < 0
        || to < 0
        || from >= settings.len() as i32
        || to >= settings.len() as i32
        || from == to
    {
        return;
    }
    settings.swap(from as usize, to as usize);
    update_settings_preview(window, settings);
    select_clock(window, settings, to);
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

fn parse_color(value: &str) -> Color {
    let value = value.trim_start_matches('#');
    let rgb = match value.len() {
        6 => u32::from_str_radix(value, 16)
            .ok()
            .map(|rgb| (rgb >> 16, rgb >> 8, rgb)),
        8 => u32::from_str_radix(value, 16)
            .ok()
            .map(|argb| (argb >> 16, argb >> 8, argb)),
        _ => None,
    };

    rgb.map_or(Color::from_rgb_u8(255, 255, 255), |(red, green, blue)| {
        Color::from_rgb_u8(red as u8, green as u8, blue as u8)
    })
}

fn hsv_hex(hue: f32, saturation: f32, value: f32) -> String {
    let color = hsv_color(hue, saturation, value);
    format!(
        "#{:02X}{:02X}{:02X}",
        color.red(),
        color.green(),
        color.blue()
    )
}

fn hsv_color(hue: f32, saturation: f32, value: f32) -> Color {
    let chroma = value / 100.0 * saturation / 100.0;
    let segment = hue / 60.0;
    let secondary = chroma * (1.0 - ((segment % 2.0) - 1.0).abs());
    let (red, green, blue) = match segment as i32 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let offset = value / 100.0 - chroma;
    Color::from_rgb_u8(
        ((red + offset) * 255.0) as u8,
        ((green + offset) * 255.0) as u8,
        ((blue + offset) * 255.0) as u8,
    )
}
