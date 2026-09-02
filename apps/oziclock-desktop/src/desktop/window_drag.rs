use super::AppWindow;
use slint::{ComponentHandle, winit_030::WinitWindowAccessor};
use std::{cell::Cell, rc::Rc};

use slint::winit_030::winit::dpi::PhysicalPosition;
#[cfg(target_os = "macos")]
use std::cell::RefCell;

#[derive(Clone)]
pub(super) struct WindowDragState {
    moved: Rc<Cell<bool>>,
    start_position: Rc<Cell<Option<PhysicalPosition<i32>>>>,
    window: slint::Weak<AppWindow>,
}

impl WindowDragState {
    pub(super) fn take_moved(&self) -> bool {
        let moved_during_drag = self.moved.replace(false);
        let Some(start) = self.start_position.replace(None) else {
            return moved_during_drag;
        };
        let mut current = None;
        if let Some(window) = self.window.upgrade() {
            let _ = window.window().with_winit_window(|native| {
                current = native.outer_position().ok();
            });
        }
        moved_during_drag || position_changed(Some(start), current)
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
struct MacOsDragAnchor {
    x: f64,
    y: f64,
}

#[cfg(target_os = "macos")]
pub(super) fn configure_main_window_drag(window: &AppWindow) -> WindowDragState {
    let drag_anchor = Rc::new(RefCell::new(None::<MacOsDragAnchor>));
    let moved = Rc::new(Cell::new(false));
    let start_position = Rc::new(Cell::new(None));
    let start_window = window.as_weak();
    let start_anchor = drag_anchor.clone();
    let moved_on_start = moved.clone();
    let position_on_start = start_position.clone();
    window.on_request_window_drag_start(move |x, y| {
        moved_on_start.set(false);
        if let Some(window) = start_window.upgrade() {
            let _ = window.window().with_winit_window(|native| {
                position_on_start.set(native.outer_position().ok());
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
    let moved_on_move = moved.clone();
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
            moved_on_move.set(true);
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
    WindowDragState {
        moved,
        start_position,
        window: window.as_weak(),
    }
}

#[cfg(not(target_os = "macos"))]
pub(super) fn configure_main_window_drag(window: &AppWindow) -> WindowDragState {
    let moved = Rc::new(Cell::new(false));
    let start_position = Rc::new(Cell::new(None));
    let drag_window = window.as_weak();
    let moved_after_drag = moved.clone();
    let position_on_start = start_position.clone();
    window.on_request_window_drag_start(move |_x, _y| {
        moved_after_drag.set(false);
        if let Some(window) = drag_window.upgrade() {
            let _ = window.window().with_winit_window(|native| {
                position_on_start.set(native.outer_position().ok());
                let _ = native.drag_window();
            });
        }
    });
    window.on_request_window_drag_move(|_x, _y| {});
    window.on_request_window_drag_end(|| {});
    WindowDragState {
        moved,
        start_position,
        window: window.as_weak(),
    }
}

fn position_changed(
    before: Option<PhysicalPosition<i32>>,
    after: Option<PhysicalPosition<i32>>,
) -> bool {
    matches!((before, after), (Some(before), Some(after)) if before != after)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_only_a_real_window_move() {
        let origin = PhysicalPosition::new(100, 200);
        assert!(!position_changed(Some(origin), Some(origin)));
        assert!(position_changed(
            Some(origin),
            Some(PhysicalPosition::new(101, 200))
        ));
        assert!(!position_changed(None, Some(origin)));
    }
}
