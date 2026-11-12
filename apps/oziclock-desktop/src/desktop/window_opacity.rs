use super::{AppWindow, clamp_inactive_opacity};
use objc2_app_kit::NSView;
use slint::{
    ComponentHandle,
    winit_030::{
        WinitWindowAccessor,
        winit::raw_window_handle::{HasWindowHandle, RawWindowHandle},
    },
};

pub(super) fn configure(window: &AppWindow) {
    window.set_native_opacity(true);
    let weak = window.as_weak();
    window.on_request_native_opacity(move |opacity| {
        if let Some(window) = weak.upgrade() {
            apply(&window, opacity);
        }
    });
}

pub(super) fn sync(window: &AppWindow) {
    apply(window, window.get_effective_opacity());
}

fn apply(window: &AppWindow, opacity: f32) {
    let _ = window.window().with_winit_window(|native| {
        let Ok(handle) = native.window_handle() else {
            return;
        };
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return;
        };
        // Winit supplies a live NSView. Startup and Slint callbacks run on the
        // main UI thread, and the borrowed view never escapes this closure.
        let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
        if let Some(native_window) = view.window() {
            native_window.setAlphaValue(clamp_inactive_opacity(f64::from(opacity)));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::clamp_inactive_opacity;

    #[test]
    fn win_06a_macos_opacity_has_a_5_1_percent_floor() {
        for legacy_opacity in [0.02, 0.04, 0.05] {
            assert_eq!(clamp_inactive_opacity(legacy_opacity), 0.051);
        }
        let slider_minimum = 5.1_f32;
        assert_eq!(
            clamp_inactive_opacity(f64::from(slider_minimum) / 100.0),
            0.051
        );
        assert_eq!(clamp_inactive_opacity(0.13), 0.13);
        assert_eq!(clamp_inactive_opacity(1.0), 1.0);
    }
}
