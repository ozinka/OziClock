use super::{AppWindow, RulersWindow, TimeSliderWindow};
use slint::winit_030::winit::dpi::PhysicalPosition;
use slint::{ComponentHandle, winit_030::WinitWindowAccessor};

#[cfg(target_os = "macos")]
use std::{cell::RefCell, rc::Rc};

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
struct MacOsDragAnchor {
    x: f64,
    y: f64,
}

#[cfg(target_os = "macos")]
pub(super) fn configure_main_window_drag(
    window: &AppWindow,
    _rulers_window: slint::Weak<RulersWindow>,
    _time_slider_window: slint::Weak<TimeSliderWindow>,
) {
    let drag_anchor = Rc::new(RefCell::new(None::<MacOsDragAnchor>));
    let start_window = window.as_weak();
    let start_anchor = drag_anchor.clone();
    window.on_request_window_drag_start(move |x, y| {
        if let Some(window) = start_window.upgrade() {
            let _ = window.window().with_winit_window(|native| {
                let scale = native.scale_factor();
                *start_anchor.borrow_mut() = Some(MacOsDragAnchor {
                    x: f64::from(x) * scale,
                    y: f64::from(y) * scale,
                });
            });
        }
    });

    let move_window = window.as_weak();
    let move_anchor = drag_anchor.clone();
    window.on_request_window_drag_move(move |x, y| {
        let Some(anchor) = *move_anchor.borrow() else {
            return;
        };
        let Some(window) = move_window.upgrade() else {
            return;
        };
        let _ = window.window().with_winit_window(|native| {
            let scale = native.scale_factor();
            let delta_x = (f64::from(x) * scale - anchor.x).round() as i32;
            let delta_y = (f64::from(y) * scale - anchor.y).round() as i32;
            if delta_x == 0 && delta_y == 0 {
                return;
            }
            let Ok(position) = native.outer_position() else {
                return;
            };
            native.set_outer_position(PhysicalPosition::new(
                position.x + delta_x,
                position.y + delta_y,
            ));
        });
    });

    let end_anchor = drag_anchor;
    window.on_request_window_drag_end(move || {
        *end_anchor.borrow_mut() = None;
    });
}

#[cfg(not(target_os = "macos"))]
pub(super) fn configure_main_window_drag(
    window: &AppWindow,
    _rulers_window: slint::Weak<RulersWindow>,
    _time_slider_window: slint::Weak<TimeSliderWindow>,
) {
    let drag_window = window.as_weak();
    window.on_request_window_drag_start(move |_x, _y| {
        if let Some(window) = drag_window.upgrade() {
            let _ = window
                .window()
                .with_winit_window(|native| native.drag_window());
        }
    });
    window.on_request_window_drag_move(|_x, _y| {});
    window.on_request_window_drag_end(|| {});
}

pub(super) fn position_attached_windows(
    main_window: &slint::Weak<AppWindow>,
    rulers_window: &slint::Weak<RulersWindow>,
    time_slider_window: &slint::Weak<TimeSliderWindow>,
) {
    let (Some(main_window), Some(rulers_window), Some(time_slider_window)) = (
        main_window.upgrade(),
        rulers_window.upgrade(),
        time_slider_window.upgrade(),
    ) else {
        return;
    };
    let _ = main_window.window().with_winit_window(|main_native| {
        let Ok(main_position) = main_native.outer_position() else {
            return;
        };
        let main_height = main_native.inner_size().height as i32;
        let ruler_height =
            (463.0 * main_window.get_clock_scale() * main_native.scale_factor() as f32).round()
                as i32;
        let _ = rulers_window.window().with_winit_window(|ruler_native| {
            ruler_native.set_outer_position(PhysicalPosition::new(
                main_position.x,
                main_position.y + main_height,
            ));
        });
        let _ = time_slider_window
            .window()
            .with_winit_window(|slider_native| {
                slider_native.set_outer_position(PhysicalPosition::new(
                    main_position.x,
                    main_position.y + main_height + ruler_height,
                ));
            });
    });
}
