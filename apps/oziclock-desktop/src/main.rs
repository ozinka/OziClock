#![windows_subsystem = "windows"]

slint::include_modules!();

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use oziclock_storage::ClockSettings;
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::dpi::PhysicalPosition;
use slint::{Color, ModelRc, Timer, TimerMode, VecModel};

fn main() -> Result<(), slint::PlatformError> {
    let settings = oziclock_storage::load_or_initialize().map_err(|error| {
        slint::PlatformError::Other(format!("could not load OziClock settings: {error}"))
    })?;
    let window = AppWindow::new()?;
    window.set_product_name(oziclock_app::application_name().into());
    window.set_show_seconds(settings.show_seconds);
    update_clock_tiles(&window, &settings.clocks_settings, settings.show_seconds);

    let settings_window = SettingsWindow::new()?;
    update_settings_preview(&settings_window, &settings.clocks_settings);
    select_clock(&settings_window, &settings.clocks_settings, 0);
    let shared_settings = Rc::new(RefCell::new(settings));
    let weak_settings_window = settings_window.as_weak();
    window.on_request_open_settings(move || {
        if let Some(settings_window) = weak_settings_window.upgrade() {
            let _ = settings_window.show();
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
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
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
    });
    let state = shared_settings.clone();
    let editor = settings_window.as_weak();
    settings_window.on_request_remove(move || {
        if let Some(editor) = editor.upgrade() {
            let mut state = state.borrow_mut();
            let index = editor.get_selected_index();
            if state.clocks_settings.len() > 1 && index >= 0 {
                state.clocks_settings.remove(index as usize);
                update_settings_preview(&editor, &state.clocks_settings);
                select_clock(&editor, &state.clocks_settings, 0);
            }
        }
    });
    let state = shared_settings.clone();
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
    let editor = settings_window.as_weak();
    let drag_start = drag_index.clone();
    settings_window.on_request_drag_start(move |index| {
        drag_start.set(index);
        if let Some(editor) = editor.upgrade() {
            editor.set_dragging_index(index);
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
    settings_window.on_request_drag_end(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_dragging_index(-1);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_open_color_picker(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_color_picker_open(true);
        }
    });
    let editor = settings_window.as_weak();
    settings_window.on_request_pick_color(move |color| {
        if let Some(editor) = editor.upgrade() {
            editor.set_editor_color(color);
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
            match oziclock_storage::save(&state) {
                Ok(()) => editor.set_status_message("Saved beside the executable.".into()),
                Err(error) => editor.set_status_message(format!("Save failed: {error}").into()),
            };
            update_settings_preview(&editor, &state.clocks_settings);
            if let Some(main_window) = main_window.upgrade() {
                update_clock_tiles(&main_window, &state.clocks_settings, state.show_seconds);
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
    context_menu.on_request_open_settings(move || {
        if let Some(context_menu) = weak_context_menu.upgrade() {
            let _ = context_menu.hide();
        }
        if let Some(settings_window) = weak_settings_window.upgrade() {
            let _ = settings_window.show();
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
        }
    });
    context_menu.on_request_exit(|| {
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
    let state = shared_settings;
    let weak_window = window.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        if let Some(window) = weak_window.upgrade() {
            let state = state.borrow();
            update_clock_tiles(&window, &state.clocks_settings, state.show_seconds);
        }
    });

    window.run()
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
        window.set_selected_index(index.max(0));
        window.set_editor_label(clock.label.clone().into());
        window.set_editor_time_zone(clock.time_zone.clone().into());
        window.set_editor_color(clock.color.clone().into());
        window.set_editor_is_main(clock.is_main);
        window.set_status_message("".into());
    }
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
    let requested_x = owner.get_menu_x();
    let _ = owner.window().with_winit_window(|winit_owner| {
        let owner_position = winit_owner.outer_position().unwrap_or_default();
        let scale_factor = winit_owner.scale_factor();
        let maximum_x =
            winit_owner.inner_size().width.saturating_sub(216) as f32 / scale_factor as f32;
        let x = requested_x.min(maximum_x).max(0.0);
        let menu_position = PhysicalPosition::new(
            owner_position.x + (x * scale_factor as f32) as i32,
            owner_position.y + winit_owner.outer_size().height as i32 + 4,
        );
        let _ = context_menu
            .window()
            .with_winit_window(|menu| menu.set_outer_position(menu_position));
    });
}

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
