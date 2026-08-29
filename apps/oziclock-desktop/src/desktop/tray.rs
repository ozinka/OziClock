use super::AppWindow;
use slint::ComponentHandle;
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

pub(super) struct SystemTray {
    _icon: TrayIcon,
}

pub(super) fn create_system_tray(
    main_window: slint::Weak<AppWindow>,
    top_most: bool,
) -> Result<SystemTray, slint::PlatformError> {
    let menu = Menu::new();
    let show_hide = MenuItem::with_id("show-hide", "Show/Hide", true, None);
    let open_settings = MenuItem::with_id("settings", "Settings", true, None);
    let always_on_top =
        CheckMenuItem::with_id("always-on-top", "Always on top", true, top_most, None);
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

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let main_window = main_window.clone();
        let _ = slint::invoke_from_event_loop(move || match event.id().0.as_str() {
            "show-hide" => {
                if let Some(main_window) = main_window.upgrade() {
                    if main_window.window().is_visible() {
                        let _ = main_window.hide();
                    } else {
                        let _ = main_window.show();
                        main_window.invoke_request_refresh_time();
                    }
                }
            }
            "settings" => {
                if let Some(main_window) = main_window.upgrade() {
                    main_window.invoke_request_open_settings();
                }
            }
            "always-on-top" => {
                if let Some(main_window) = main_window.upgrade() {
                    main_window.invoke_request_toggle_top_most();
                }
            }
            "exit" => {
                if let Some(main_window) = main_window.upgrade() {
                    main_window.invoke_request_tray_exit();
                }
            }
            _ => {}
        });
    }));

    Ok(SystemTray { _icon: icon })
}
