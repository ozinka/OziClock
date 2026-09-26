//! Apply the persisted theme to auxiliary windows and the clock's time slider.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn bind(
    clock: &AppWindow,
    editor: &SettingsWindow,
    calendar: &CalendarWindow,
    planner: &PlannerWindow,
    about: &AboutWindow,
    menu: &ContextMenuWindow,
    alarm: &AlarmAttentionWindow,
    timer: &TimerAttentionWindow,
    reminder: &ReminderAttentionWindow,
    event: &EventAttentionWindow,
    task: &TaskAttentionWindow,
    settings: Rc<RefCell<AppSettings>>,
) {
    let clock = clock.as_weak();
    let editor_weak = editor.as_weak();
    let calendar = calendar.as_weak();
    let planner = planner.as_weak();
    let about = about.as_weak();
    let menu = menu.as_weak();
    let alarm = alarm.as_weak();
    let timer = timer.as_weak();
    let reminder = reminder.as_weak();
    let event = event.as_weak();
    let task = task.as_weak();
    let state = settings.clone();
    editor.on_application_theme_changed(move || {
        let state = state.borrow();
        let light = state.application_light_theme;
        macro_rules! apply {
            ($window:expr) => {
                if let Some(window) = $window.upgrade() {
                    let palette = window.global::<AppPalette>();
                    palette.set_light(light);
                    palette.invoke_apply_theme();
                }
            };
        }
        apply!(clock);
        apply!(editor_weak);
        apply!(calendar);
        apply!(about);
        apply!(menu);
        apply!(planner);
        apply!(alarm);
        apply!(timer);
        apply!(reminder);
        apply!(event);
        apply!(task);
        if let Some(editor) = editor_weak.upgrade() {
            editor.set_application_light_theme(light);
        }
    });
    let editor_weak = editor.as_weak();
    editor.on_request_set_application_light_theme(move |light| {
        settings.borrow_mut().application_light_theme = light;
        if let Some(editor) = editor_weak.upgrade() {
            editor.invoke_application_theme_changed();
        }
    });
    editor.invoke_application_theme_changed();
}
