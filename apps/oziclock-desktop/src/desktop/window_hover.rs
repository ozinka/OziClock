use super::AppWindow;
use objc2::MainThreadOnly;
use objc2_app_kit::{NSView, NSWindow, NSWindowOcclusionState};
use slint::{
    ComponentHandle, Timer, TimerMode,
    winit_030::{
        WinitWindowAccessor,
        winit::{
            event::WindowEvent,
            raw_window_handle::{HasWindowHandle, RawWindowHandle},
        },
    },
};
use std::{rc::Rc, time::Duration};

#[derive(Clone)]
pub(super) struct HoverMonitor {
    window: slint::Weak<AppWindow>,
    timer: Rc<Timer>,
}

impl HoverMonitor {
    pub(super) fn new(window: &AppWindow) -> Self {
        window.set_use_native_strip_hover(true);
        Self {
            window: window.as_weak(),
            timer: Rc::new(Timer::default()),
        }
    }

    pub(super) fn on_window_event(&self, event: &WindowEvent) {
        match event {
            WindowEvent::Occluded(true) | WindowEvent::Destroyed => self.set_hovered(false),
            WindowEvent::CursorMoved { .. }
            | WindowEvent::CursorEntered { .. }
            | WindowEvent::CursorLeft { .. }
            | WindowEvent::Focused(_)
            | WindowEvent::Occluded(false)
            | WindowEvent::Moved(_)
            | WindowEvent::Resized(_)
            | WindowEvent::ScaleFactorChanged { .. } => self.sync(),
            _ => {}
        }
    }

    pub(super) fn sync(&self) {
        let Some(window) = self.window.upgrade() else {
            self.timer.stop();
            return;
        };
        if let Some(hovered) = native_strip_hovered(&window) {
            self.set_hovered(hovered);
        }
    }

    fn set_hovered(&self, hovered: bool) {
        if let Some(window) = self.window.upgrade() {
            window.set_native_strip_hovered(hovered);
        }
        if !hovered {
            self.timer.stop();
        } else if !self.timer.running() {
            let window = self.window.clone();
            let timer = Rc::downgrade(&self.timer);
            // Recover even if no tracking-area exit reaches the UI. Recheck only
            // while hovered; an inactive strip has no polling timer.
            self.timer
                .start(TimerMode::Repeated, Duration::from_millis(250), move || {
                    if let Some(timer) = timer.upgrade() {
                        Self {
                            window: window.clone(),
                            timer,
                        }
                        .sync();
                    }
                });
        }
    }
}

fn native_strip_hovered(window: &AppWindow) -> Option<bool> {
    if window.get_modal_open() {
        return Some(false);
    }
    window
        .window()
        .with_winit_window(|native| {
            let handle = native.window_handle().ok()?;
            let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
                return None;
            };
            // Winit owns this live view. Event hooks and timers run on the main UI
            // thread, and no native reference escapes the closure.
            let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
            let native_window = view.window()?;
            if !native_window.isVisible()
                || native_window.isMiniaturized()
                || !native_window
                    .occlusionState()
                    .contains(NSWindowOcclusionState::Visible)
            {
                return Some(false);
            }
            let window_point = native_window.mouseLocationOutsideOfEventStream();
            let point = view.convertPoint_fromView(window_point, None);
            let bounds = view.bounds();
            let x = point.x - bounds.origin.x;
            let y = if view.isFlipped() {
                point.y - bounds.origin.y
            } else {
                bounds.origin.y + bounds.size.height - point.y
            };
            let strip_height = f64::from(window.get_clock_viewport_height())
                * f64::from(window.window().scale_factor())
                / native.scale_factor();
            if !contains_strip(x, y, bounds.size.width, strip_height) {
                return Some(false);
            }
            // Geometric overlap alone must not count as hovering through another
            // application's window or one of our auxiliary windows.
            let hit_window = NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(
                native_window.convertPointToScreen(window_point),
                0,
                native_window.mtm(),
            );
            Some(hit_window == native_window.windowNumber())
        })
        .flatten()
}

fn contains_strip(x: f64, y: f64, width: f64, height: f64) -> bool {
    x >= 0.0 && x < width && y >= 0.0 && y < height
}

#[cfg(test)]
mod tests {
    use super::contains_strip;

    #[test]
    fn clk_14_strip_bounds_exclude_all_four_outside_edges() {
        assert!(contains_strip(100.0, 30.0, 300.0, 62.0));
        for (x, y) in [(-0.1, 30.0), (300.0, 30.0), (100.0, -0.1), (100.0, 62.0)] {
            assert!(!contains_strip(x, y, 300.0, 62.0));
        }
    }

    #[test]
    fn clk_14_compact_strip_excludes_rulers_and_accepts_fractional_coordinates() {
        assert!(contains_strip(0.0, 0.0, 300.0, 31.0));
        assert!(contains_strip(299.75, 30.75, 300.0, 31.0));
        assert!(!contains_strip(100.0, 31.0, 300.0, 31.0));
        assert!(!contains_strip(100.0, 300.0, 300.0, 31.0));
    }
}
