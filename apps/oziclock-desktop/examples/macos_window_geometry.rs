//! BL-013 native regression probe. Run on macOS with a graphical login session:
//! cargo run -p oziclock-desktop --example macos_window_geometry
//! Timers separate event-loop turns; they never correct a tested window's position.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This native regression probe requires macOS.");
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), slint::PlatformError> {
    probe::run()
}

#[cfg(target_os = "macos")]
mod probe {
    use objc2_app_kit::NSView;
    use slint::winit_030::{
        WinitWindowAccessor,
        winit::{
            dpi::{LogicalPosition, LogicalSize},
            raw_window_handle::{HasWindowHandle, RawWindowHandle},
        },
    };
    use slint::{ComponentHandle, Timer, TimerMode};
    use std::{cell::Cell, rc::Rc, time::Duration};

    slint::slint! {
        export component GeometryWindow inherits Window {
            in property <bool> frameless: true;
            no-frame: frameless;
            always-on-top: true;
            preferred-width: 601px;
            preferred-height: 31px;
            background: #803060;
        }
    }

    fn assert_read_only(window: &GeometryWindow) {
        window
            .window()
            .with_winit_window(|native| {
                let handle = native.window_handle().unwrap();
                let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
                    unreachable!()
                };
                // Winit owns this view; the callback borrows it on the main thread.
                let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
                let ns_window = view.window().unwrap();
                let frame = ns_window.frame();
                let style = ns_window.styleMask();
                for _ in 0..100 {
                    let _ = native.is_maximized();
                    assert_eq!(
                        ns_window.frame(),
                        frame,
                        "WIN-01: state query moved the window"
                    );
                    assert_eq!(
                        ns_window.styleMask(),
                        style,
                        "WIN-01: state query changed style"
                    );
                }
            })
            .expect("native window exists");
    }

    pub fn run() -> Result<(), slint::PlatformError> {
        let initial = Rc::new(Cell::new(LogicalPosition::new(200.0, 100.0)));
        let hook_position = initial.clone();
        slint::BackendSelector::new()
            .with_winit_window_attributes_hook(move |a| {
                a.with_position(hook_position.get())
                    .with_inner_size(LogicalSize::new(601.0, 31.0))
                    .with_decorations(false)
            })
            .select()?;
        let bootstrap = GeometryWindow::new()?;
        bootstrap.show()?;
        let bootstrap_weak = bootstrap.as_weak();
        let mut subject: Option<GeometryWindow> = None;
        let mut decorated: Option<GeometryWindow> = None;
        let mut origin = None;
        let mut restore_frame = None;
        let mut step = 0;
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, Duration::from_millis(80), move || {
            if step == 0 {
                let bootstrap = bootstrap_weak.upgrade().unwrap();
                bootstrap.window().with_winit_window(|native| {
                    let handle = native.window_handle().unwrap();
                    let RawWindowHandle::AppKit(handle) = handle.as_raw() else { unreachable!() };
                    // The view and its screen are read only on the main thread.
                    let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
                    let ns_window = view.window().unwrap();
                    let frame = ns_window.frame();
                    let area = ns_window.screen().unwrap().visibleFrame();
                    let position = native.outer_position().unwrap().to_logical::<f64>(native.scale_factor());
                    let top = position.y + frame.origin.y + frame.size.height - area.origin.y - area.size.height;
                    initial.set(LogicalPosition::new(position.x, top));
                }).unwrap();
                let w = GeometryWindow::new().unwrap();
                w.show().unwrap();
                subject = Some(w);
            } else {
                let w = subject.as_ref().unwrap();
                assert_read_only(w);
                if step <= 25 {
                    w.window().with_winit_window(|native| {
                        let expected = initial.get().to_physical::<i32>(native.scale_factor());
                        assert_eq!(native.outer_position().unwrap(), expected, "WIN-03/MODE-04: startup or resize shifted top edge at step {step}");
                        origin = Some(expected);
                        // Alternate compact, standard and ruler heights at 85% and 100%.
                        let scale = if step % 2 == 0 { 0.85 } else { 1.0 };
                        let height = [31.0, 62.0, 563.0, 594.0][step % 4] * scale;
                        let _ = native.request_inner_size(LogicalSize::new(601.0 * scale, height));
                    }).unwrap();
                } else if step == 26 {
                    w.window().with_winit_window(|native| {
                        assert_eq!(native.outer_position().unwrap(), origin.unwrap());
                        restore_frame = Some((native.outer_position().unwrap(), native.outer_size()));
                        native.set_maximized(true);
                    }).unwrap();
                } else if step == 27 {
                    w.window().with_winit_window(|native| {
                        assert!(native.is_maximized(), "borderless maximize must remain supported");
                        native.set_maximized(false);
                    }).unwrap();
                } else if step == 28 {
                    w.window().with_winit_window(|native| {
                        assert!(!native.is_maximized());
                        assert_eq!((native.outer_position().unwrap(), native.outer_size()), restore_frame.unwrap(), "maximize/restore must preserve the original frame");
                        native.set_minimized(true);
                    }).unwrap();
                } else if step == 29 {
                    w.window().with_winit_window(|native| native.set_minimized(false)).unwrap();
                } else if step == 30 {
                    w.window().with_winit_window(|native| {
                        assert_eq!(native.outer_position().unwrap(), origin.unwrap(), "restoring a minimized window shifted it");
                    }).unwrap();
                    initial.set(LogicalPosition::new(200.0, 200.0));
                    let d = GeometryWindow::new().unwrap();
                    d.set_frameless(false);
                    d.show().unwrap();
                    decorated = Some(d);
                } else if step == 31 {
                    let d = decorated.as_ref().unwrap();
                    assert_read_only(d);
                    d.window().with_winit_window(|native| {
                        restore_frame = Some((native.outer_position().unwrap(), native.outer_size()));
                        native.set_maximized(true);
                    }).unwrap();
                } else if step == 32 {
                    let d = decorated.as_ref().unwrap();
                    assert_read_only(d);
                    d.window().with_winit_window(|native| {
                        assert!(native.is_maximized(), "decorated maximize must remain supported");
                        native.set_maximized(false);
                    }).unwrap();
                } else {
                    let d = decorated.as_ref().unwrap();
                    assert_read_only(d);
                    d.window().with_winit_window(|native| {
                        assert_eq!((native.outer_position().unwrap(), native.outer_size()), restore_frame.unwrap());
                    }).unwrap();
                    println!("PASS: WIN-01/WIN-03/MODE-04 startup, 85%/100% resize cycles, read-only state queries, borderless/decorated maximize-restore, minimize-restore");
                    slint::quit_event_loop().unwrap();
                }
            }
            step += 1;
        });
        bootstrap.run()
    }
}
