use super::*;
use oziclock_storage::PlannerAppearance;

fn resolved_accent(appearance: &PlannerAppearance, clock_accent: slint::Color) -> slint::Color {
    if appearance.follow_main_clock {
        clock_accent.brighter(0.4)
    } else {
        valid_color(&appearance.custom_accent, "#77D7CB")
    }
}

fn valid_color(value: &str, fallback: &str) -> slint::Color {
    parse_color(&normalize_border_color(value).unwrap_or_else(|| fallback.into()))
}

fn type_colors(appearance: &PlannerAppearance, accent: slint::Color) -> [slint::Color; 5] {
    if appearance.use_accent_color {
        return [accent; 5];
    }
    [
        valid_color(&appearance.alarm_color, "#77D7CB"),
        valid_color(&appearance.timer_color, "#77D7CB"),
        valid_color(&appearance.reminder_color, "#77D7CB"),
        valid_color(&appearance.event_color, "#B99655"),
        valid_color(&appearance.task_color, "#547D78"),
    ]
}

pub(super) fn calendar_indicator_colors(settings: &AppSettings) -> [slint::Color; 4] {
    let accent = resolved_accent(&settings.planner_appearance, calendar_accent(settings));
    let colors = type_colors(&settings.planner_appearance, accent);
    [colors[3], colors[2], colors[4], colors[0]]
}

pub(super) fn refresh(planner: &PlannerWindow, settings: &AppSettings) {
    let accent = resolved_accent(&settings.planner_appearance, calendar_accent(settings));
    let colors = type_colors(&settings.planner_appearance, accent);
    let palette = planner.global::<PlannerPalette>();
    palette.set_light(settings.planner_appearance.light_theme);
    palette.invoke_apply_theme();
    palette.set_accent(accent);
    palette.set_accent_foreground(accent_foreground(accent));
    palette.set_reminder_color(colors[2]);
    palette.set_reminder_color_foreground(accent_foreground(colors[2]));
    palette.set_event_color(colors[3]);
    palette.set_event_color_foreground(accent_foreground(colors[3]));
    palette.set_task_color(colors[4]);
    palette.set_task_color_foreground(accent_foreground(colors[4]));
    planner.set_accent(accent);
    planner.invoke_appearance_changed(
        settings.planner_appearance.light_theme,
        colors[0],
        colors[1],
        colors[2],
        colors[3],
        colors[4],
    );
}

fn refresh_editor(editor: &SettingsWindow, appearance: &PlannerAppearance) {
    editor.set_planner_light_theme(appearance.light_theme);
    editor.set_planner_follow_main_clock(appearance.follow_main_clock);
    editor.set_planner_use_accent_color(appearance.use_accent_color);
    let values = [
        &appearance.custom_accent,
        &appearance.alarm_color,
        &appearance.timer_color,
        &appearance.reminder_color,
        &appearance.event_color,
        &appearance.task_color,
    ];
    editor.set_planner_colors(
        Rc::new(slint::VecModel::from(
            values
                .iter()
                .map(|value| slint::SharedString::from(value.as_str()))
                .collect::<Vec<_>>(),
        ))
        .into(),
    );
    editor.set_planner_color_previews(
        Rc::new(slint::VecModel::from(
            values
                .iter()
                .map(|value| valid_color(value, "#77D7CB"))
                .collect::<Vec<_>>(),
        ))
        .into(),
    );
}

// All attention windows subscribe to the same refresh path as live clock previews.
#[allow(clippy::too_many_arguments)]
pub(super) fn bind(
    editor: &SettingsWindow,
    planner: &PlannerWindow,
    alarm: &AlarmAttentionWindow,
    timer: &TimerAttentionWindow,
    reminder: &ReminderAttentionWindow,
    event: &EventAttentionWindow,
    task: &TaskAttentionWindow,
    settings: Rc<RefCell<AppSettings>>,
) {
    let alarm = alarm.as_weak();
    let timer = timer.as_weak();
    let reminder = reminder.as_weak();
    let event = event.as_weak();
    let task = task.as_weak();
    planner.on_appearance_changed(
        move |light, alarm_color, timer_color, reminder_color, event_color, task_color| {
            let colors = [
                alarm_color,
                timer_color,
                reminder_color,
                event_color,
                task_color,
            ];
            macro_rules! update {
                ($window:expr, $index:expr) => {
                    if let Some(window) = $window.upgrade() {
                        window.set_accent(colors[$index]);
                        let palette = window.global::<PlannerPalette>();
                        palette.set_light(light);
                        palette.invoke_apply_theme();
                        palette.set_accent_foreground(accent_foreground(colors[$index]));
                    }
                };
            }
            update!(alarm, 0);
            update!(timer, 1);
            update!(reminder, 2);
            update!(event, 3);
            update!(task, 4);
        },
    );
    refresh_editor(editor, &settings.borrow().planner_appearance);
    refresh(planner, &settings.borrow());

    let state = settings.clone();
    let planner_weak = planner.as_weak();
    let editor_weak = editor.as_weak();
    editor.on_request_planner_option(move |option, enabled| {
        {
            let mut state = state.borrow_mut();
            match option {
                0 => state.planner_appearance.light_theme = enabled,
                1 => state.planner_appearance.follow_main_clock = enabled,
                2 => state.planner_appearance.use_accent_color = enabled,
                _ => return,
            }
        }
        if let Some(editor) = editor_weak.upgrade() {
            refresh_editor(&editor, &state.borrow().planner_appearance);
        }
        if let Some(planner) = planner_weak.upgrade() {
            refresh(&planner, &state.borrow());
        }
    });
    let planner = planner.as_weak();
    let editor_weak = editor.as_weak();
    editor.on_request_planner_color(move |index, value| {
        let Some(editor) = editor_weak.upgrade() else {
            return;
        };
        let Some(color) = normalize_border_color(value.as_str()) else {
            editor.set_status_message("Enter a color as #RRGGBB.".into());
            return;
        };
        {
            let mut state = settings.borrow_mut();
            let appearance = &mut state.planner_appearance;
            let target = match index {
                0 => &mut appearance.custom_accent,
                1 => &mut appearance.alarm_color,
                2 => &mut appearance.timer_color,
                3 => &mut appearance.reminder_color,
                4 => &mut appearance.event_color,
                5 => &mut appearance.task_color,
                _ => return,
            };
            *target = color;
        }
        editor.set_status_message("".into());
        refresh_editor(&editor, &settings.borrow().planner_appearance);
        if let Some(planner) = planner.upgrade() {
            refresh(&planner, &settings.borrow());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_09_custom_accent_is_independent_of_main_clock() {
        let mut appearance = PlannerAppearance {
            follow_main_clock: false,
            custom_accent: "#123456".into(),
            ..Default::default()
        };
        let red = parse_color("#FF0000");
        let blue = parse_color("#0000FF");
        assert_eq!(resolved_accent(&appearance, red), parse_color("#123456"));
        assert_eq!(resolved_accent(&appearance, blue), parse_color("#123456"));
        appearance.follow_main_clock = true;
        assert_eq!(resolved_accent(&appearance, red), red.brighter(0.4));
        assert_eq!(resolved_accent(&appearance, blue), blue.brighter(0.4));
    }

    #[test]
    fn set_10_accent_override_preserves_individual_colors() {
        let mut appearance = PlannerAppearance::default();
        let accent = parse_color("#123456");
        assert_eq!(type_colors(&appearance, accent), [accent; 5]);
        appearance.use_accent_color = false;
        let individual = type_colors(&appearance, accent);
        assert_eq!(individual[3], parse_color("#B99655"));
        assert_eq!(individual[4], parse_color("#547D78"));
        appearance.use_accent_color = true;
        assert_eq!(type_colors(&appearance, accent), [accent; 5]);
        appearance.use_accent_color = false;
        assert_eq!(type_colors(&appearance, accent), individual);
        appearance.event_color = "invalid".into();
        assert_eq!(type_colors(&appearance, accent)[3], parse_color("#B99655"));
    }
}
