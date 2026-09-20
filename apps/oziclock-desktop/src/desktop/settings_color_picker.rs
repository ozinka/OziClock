use super::*;

#[derive(Clone, Copy)]
enum ColorTarget {
    Clock,
    Border,
    Planner(i32),
}

fn apply_color(editor: &SettingsWindow, target: ColorTarget, color: slint::SharedString) {
    editor.set_picker_color_value(color.clone());
    match target {
        ColorTarget::Clock => {
            editor.set_editor_preview_color(parse_color(&color));
            editor.set_editor_color(color);
            editor.invoke_request_apply();
        }
        ColorTarget::Border => {
            editor.set_border_preview_color(parse_color(&color));
            editor.set_border_color_value(color.clone());
            editor.invoke_request_set_border_color(color);
        }
        ColorTarget::Planner(index) => editor.invoke_request_planner_color(index, color),
    }
}

fn open_picker(
    editor: &SettingsWindow,
    target: &Cell<ColorTarget>,
    original: &RefCell<slint::SharedString>,
    selected: ColorTarget,
    color: slint::SharedString,
) {
    target.set(selected);
    *original.borrow_mut() = color.clone();
    let (hue, saturation, value) = color_to_hsv(&color);
    editor.set_picker_hue(hue);
    editor.set_picker_saturation(saturation);
    editor.set_picker_value(value);
    editor.set_picker_hue_color(hsv_color(hue, 100.0, 100.0));
    editor.set_picker_color_value(color);
    editor.set_color_picker_open(true);
}

pub(super) fn bind(window: &SettingsWindow) {
    let target = Rc::new(Cell::new(ColorTarget::Clock));
    let original = Rc::new(RefCell::new(slint::SharedString::default()));
    let editor = window.as_weak();
    let selected = target.clone();
    let pending = original.clone();
    window.on_request_open_color_picker(move || {
        if let Some(editor) = editor.upgrade() {
            open_picker(
                &editor,
                &selected,
                &pending,
                ColorTarget::Clock,
                editor.get_editor_color(),
            );
        }
    });
    let editor = window.as_weak();
    let selected = target.clone();
    let pending = original.clone();
    window.on_request_open_border_color_picker(move || {
        if let Some(editor) = editor.upgrade() {
            open_picker(
                &editor,
                &selected,
                &pending,
                ColorTarget::Border,
                editor.get_border_color_value(),
            );
        }
    });
    let editor = window.as_weak();
    let selected = target.clone();
    let pending = original.clone();
    window.on_request_open_planner_color_picker(move |index| {
        if let Some(editor) = editor.upgrade() {
            let enabled = match index {
                0 => !editor.get_planner_follow_main_clock(),
                1..=5 => !editor.get_planner_use_accent_color(),
                _ => false,
            };
            if enabled && let Some(color) = editor.get_planner_colors().row_data(index as usize) {
                open_picker(
                    &editor,
                    &selected,
                    &pending,
                    ColorTarget::Planner(index),
                    color,
                );
            }
        }
    });
    let editor = window.as_weak();
    window.on_request_color_confirm(move || {
        if let Some(editor) = editor.upgrade() {
            editor.set_color_picker_open(false);
        }
    });
    let editor = window.as_weak();
    let selected = target.clone();
    window.on_request_color_cancel(move || {
        if let Some(editor) = editor.upgrade() {
            apply_color(&editor, selected.get(), original.borrow().clone());
            editor.set_color_picker_open(false);
        }
    });
    let editor = window.as_weak();
    let selected = target.clone();
    window.on_request_pick_color(move |color| {
        if let Some(editor) = editor.upgrade() {
            apply_color(&editor, selected.get(), color);
            editor.set_color_picker_open(false);
        }
    });
    let editor = window.as_weak();
    let selected = target.clone();
    window.on_request_picker_color(move |x, y| {
        if let Some(editor) = editor.upgrade() {
            let saturation = (x / 378.0 * 100.0).clamp(0.0, 100.0);
            let value = (100.0 - y / 180.0 * 100.0).clamp(0.0, 100.0);
            editor.set_picker_saturation(saturation);
            editor.set_picker_value(value);
            apply_color(
                &editor,
                selected.get(),
                hsv_hex(editor.get_picker_hue(), saturation, value).into(),
            );
        }
    });
    let editor = window.as_weak();
    window.on_request_picker_hue(move |x| {
        if let Some(editor) = editor.upgrade() {
            let hue = (x / 378.0 * 360.0).clamp(0.0, 360.0);
            editor.set_picker_hue(hue);
            editor.set_picker_hue_color(hsv_color(hue, 100.0, 100.0));
            apply_color(
                &editor,
                target.get(),
                hsv_hex(
                    hue,
                    editor.get_picker_saturation(),
                    editor.get_picker_value(),
                )
                .into(),
            );
        }
    });
}
