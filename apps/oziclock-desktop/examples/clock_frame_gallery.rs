//! MODE-06A desktop rendering probe; does not load or save user settings.
//! cargo run -p oziclock-desktop --example clock_frame_gallery -- target/clock-frame
//! Optional second argument multiplies the application scale for geometry coverage.
//! Review PNGs before baselining; real monitor DPI transitions require desktop QA.

use slint::{Color, ComponentHandle, ModelRc, Timer, TimerMode, VecModel};
use std::{cell::Cell, fs::File, io::BufWriter, path::Path, rc::Rc, time::Duration};

slint::include_modules!();

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    count: usize,
    compact: bool,
    rulers: bool,
    radius: f32,
    scale: f32,
    seconds: bool,
    opacity: f32,
}

fn cases() -> Vec<Case> {
    let base = Case {
        name: "standard",
        count: 3,
        compact: false,
        rulers: false,
        radius: 12.0,
        scale: 1.0,
        seconds: true,
        opacity: 1.0,
    };
    vec![
        base,
        Case {
            name: "compact",
            compact: true,
            ..base
        },
        Case {
            name: "extended",
            rulers: true,
            ..base
        },
        Case {
            name: "compact-extended",
            compact: true,
            rulers: true,
            ..base
        },
        Case {
            name: "square",
            compact: true,
            radius: 0.0,
            ..base
        },
        Case {
            name: "capsule",
            compact: true,
            radius: 15.5,
            ..base
        },
        Case {
            name: "single",
            count: 1,
            compact: true,
            radius: 15.5,
            ..base
        },
        Case {
            name: "capsule-extended",
            compact: true,
            rulers: true,
            radius: 15.5,
            ..base
        },
        Case {
            name: "single-extended",
            count: 1,
            compact: true,
            rulers: true,
            radius: 15.5,
            ..base
        },
        Case {
            name: "small",
            compact: true,
            scale: 0.8,
            ..base
        },
        Case {
            name: "large",
            compact: true,
            scale: 1.5,
            ..base
        },
        Case {
            name: "no-seconds",
            compact: true,
            seconds: false,
            ..base
        },
        Case {
            name: "translucent",
            compact: true,
            opacity: 0.4,
            ..base
        },
    ]
}

fn configure(window: &AppWindow, case: Case) {
    let accents = [
        Color::from_rgb_u8(190, 235, 175),
        Color::from_rgb_u8(165, 215, 245),
        Color::from_rgb_u8(245, 195, 185),
    ];
    window.set_clocks(ModelRc::new(VecModel::from(
        (0..case.count)
            .map(|index| ClockTileData {
                label: ["Kyiv", "UTC", "Tokyo"][index % 3].into(),
                month: "09/".into(),
                day: "23".into(),
                hour: "23".into(),
                minute: "58".into(),
                second: "59".into(),
                accent: accents[index % 3],
                main_zone: index == 0,
            })
            .collect::<Vec<_>>(),
    )));
    window.set_rulers(ModelRc::new(VecModel::from(
        (0..case.count)
            .map(|index| RulerColumnData {
                accent: accents[index % 3],
            })
            .collect::<Vec<_>>(),
    )));
    window.set_clock_scale(case.scale);
    window.set_corner_radius(case.radius);
    window.set_compact_mode(case.compact);
    window.set_compact_progress(f32::from(case.compact));
    window.set_show_rulers(case.rulers);
    window.set_show_seconds(case.seconds);
    window.set_inactive_opacity(case.opacity);
    window.set_focus_column_position((case.count - 1) as f32);
    window.set_focus_progress(0.5);
    window.set_tick_indices(ModelRc::new(VecModel::from((0..=144).collect::<Vec<_>>())));
    window.set_label_hours(ModelRc::new(VecModel::from((0..=24).collect::<Vec<_>>())));
    window.set_hour_labels(ModelRc::new(VecModel::from(vec![0, 6, 12, 18, 24])));
    window.window().set_size(expected_size(window));
}

fn expected_size(window: &AppWindow) -> slint::PhysicalSize {
    let dpi = window.window().scale_factor();
    slint::PhysicalSize::new(
        (window.get_window_width() * dpi).round() as u32,
        ((window.get_clock_viewport_height()
            + if window.get_show_rulers() {
                window.get_ruler_extension_height()
            } else {
                0.0
            })
            * dpi)
            .round() as u32,
    )
}

fn mode_06a_verify_geometry(window: &AppWindow) {
    let metrics = window.get_strip_metrics();
    let layout = window.global::<ClockStripLayout>();
    let dpi = window.window().scale_factor();
    for length in [
        metrics.tile_width,
        metrics.line_width,
        metrics.end_padding,
        window.get_window_width(),
    ] {
        assert!(
            (length * dpi - (length * dpi).round()).abs() < 0.001,
            "MODE-06A: physical pixel alignment"
        );
    }
    assert!(
        metrics.line_width * dpi >= 0.999,
        "MODE-06A: visible hairline"
    );
    let mut content_width = 0.0;
    for index in 0..metrics.count {
        let left = layout.invoke_boundary(metrics.clone(), index as f32);
        let right = layout.invoke_boundary(metrics.clone(), (index + 1) as f32);
        let width = right - left - metrics.line_width;
        assert!(
            width >= metrics.tile_width - 0.001,
            "MODE-06A: preserve text area"
        );
        content_width += width;
        assert!(
            (layout.invoke_position_at(metrics.clone(), left) - index as f32).abs() < 0.001,
            "MODE-06A: focused column at its boundary"
        );
    }
    assert!(
        (content_width + (metrics.count + 1) as f32 * metrics.line_width
            - window.get_window_width())
        .abs()
            < 0.001,
        "MODE-06A: both outer borders and all joins fit"
    );
    // Drag inversion also holds between the widened first column and middle columns.
    for step in 0..=20 {
        let position = (metrics.count - 1) as f32 * step as f32 / 20.0;
        let x = layout.invoke_boundary(metrics.clone(), position);
        assert!(
            (layout.invoke_position_at(metrics.clone(), x) - position).abs() < 0.001,
            "MODE-06A: continuous column dragging"
        );
    }
}

fn capture(window: &AppWindow, case: Case, directory: &Path) -> Result<(), String> {
    mode_06a_verify_geometry(window);
    let snapshot = window.window().take_snapshot().expect("renderer snapshot");
    let width = snapshot.width();
    let height = snapshot.height();
    assert_eq!(
        (width, height),
        (window.window().size().width, window.window().size().height),
        "snapshot must capture the full physical window"
    );
    assert_eq!(
        window.window().size(),
        expected_size(window),
        "MODE-06A: capture at the requested physical display scale"
    );
    let path = directory.join(format!("{}.png", case.name));
    let mut encoder =
        png::Encoder::new(BufWriter::new(File::create(&path).unwrap()), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(snapshot.as_bytes())
        .unwrap();
    let at = |x: u32, y: u32| snapshot.as_slice()[(y * width + x) as usize];
    let alpha = 255.0 * window.get_effective_opacity();
    for (x, y) in [
        (width / 2, 0),
        (width / 2, height - 1),
        (0, height / 2),
        (width - 1, height / 2),
    ] {
        let pixel = at(x, y);
        if !(pixel.r < 8 && pixel.g < 8 && pixel.b < 8 && f32::from(pixel.a) >= alpha - 20.0) {
            return Err(format!(
                "MODE-06A: {} missing border at ({x},{y}): {pixel:?}",
                case.name
            ));
        }
    }
    if case.radius > 0.0 {
        if at(0, 0).a != 0 {
            return Err(format!(
                "MODE-06A: {} has an opaque outer corner",
                case.name
            ));
        }
        let radius = (case.radius * case.scale * window.window().scale_factor()).ceil() as u32;
        assert!(
            (0..radius).any(|y| (0..radius).any(|x| {
                let a = f32::from(at(x, y).a);
                a > 1.0 && a < alpha - 20.0
            })),
            "MODE-06A: antialiased outer curve"
        );
    }
    println!(
        "{}: {width} x {height}, DPI {}, logical width {}, geometry and frame checks passed",
        path.display(),
        window.window().scale_factor(),
        window.get_window_width()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/clock-frame".into());
    std::fs::create_dir_all(&directory)?;
    let window = Rc::new(AppWindow::new()?);
    window.set_product_name("OziClock frame verification".into());
    window.on_format_label(|_, hour| format!("{hour}:00").into());
    let scale_multiplier = std::env::args()
        .nth(2)
        .map(|s| s.parse::<f32>())
        .transpose()?
        .unwrap_or(1.0);
    if !scale_multiplier.is_finite() || scale_multiplier <= 0.0 {
        return Err("scale multiplier must be finite and positive".into());
    }
    let cases: Vec<_> = cases()
        .into_iter()
        .map(|mut case| {
            case.scale *= scale_multiplier;
            case
        })
        .collect();
    configure(&window, cases[0]);
    let capture_requested = Rc::new(Cell::new(false));
    let failure_count = Rc::new(Cell::new(0));
    let failures = failure_count.clone();
    let weak = window.as_weak();
    let mut index = 0;
    let mut capture_next = move || {
        let window = weak.upgrade().unwrap();
        if let Err(error) = capture(&window, cases[index], Path::new(&directory)) {
            eprintln!("{error}");
            failures.set(failures.get() + 1);
        }
        index += 1;
        if index == cases.len() {
            slint::quit_event_loop().unwrap();
        } else {
            let next = cases[index];
            let weak = window.as_weak();
            Timer::single_shot(Duration::ZERO, move || {
                configure(&weak.upgrade().unwrap(), next)
            });
        }
    };
    let timer = Timer::default();
    let backend = std::env::var("SLINT_BACKEND").unwrap_or_default();
    if backend.contains("software") || backend.contains("skia") {
        // These renderers produce snapshots by rendering into a fresh buffer.
        timer.start(
            TimerMode::Repeated,
            Duration::from_millis(450),
            capture_next,
        );
    } else {
        let requested = capture_requested.clone();
        window.window().set_rendering_notifier(move |state, _| {
            // FemtoVG reads the current back buffer; capture before the buffer swap.
            if matches!(state, slint::RenderingState::AfterRendering) && requested.replace(false) {
                capture_next();
            }
        })?;
        let weak = window.as_weak();
        timer.start(TimerMode::Repeated, Duration::from_millis(450), move || {
            capture_requested.set(true);
            weak.upgrade().unwrap().window().request_redraw();
        });
    }
    window.show()?;
    slint::run_event_loop()?;
    if failure_count.get() > 0 {
        return Err(format!(
            "{} frame scenarios failed; inspect the saved PNGs",
            failure_count.get()
        )
        .into());
    }
    Ok(())
}
