# Visual Testing

## Current Process

The ruler/lens UI is rendered by Slint and must be visually reviewed on a desktop build. Before a visual release, capture reviewed reference images for these cases:

- Lens at the top, center, and bottom positions.
- A non-main Column slider position.
- A time zone with a 30- or 45-minute offset.
- Standard and compact clock modes at 100%, 125%, 150%, and 200% display scale.

Compare each capture against the current approved reference in `legacy/dotnet-wpf/Ozi.Clock/Assets/` and the latest product screenshots. Check borders, one-pixel joins, label baselines, lens clipping, and attached-window alignment.

## Automation Boundary

Settings on Windows (UI-09 / BL-095): drag the top and bottom six-logical-pixel
edges and verify a vertical resize cursor, fixed 710 logical-pixel width and a
minimum height of 672. The bottom OK button and sidebar viewport follow the new
height. Close/reopen Settings and restart the application to verify persisted
height. Check header dragging, rounded corners, the color picker, and deletion
modal blocking; the invisible resize targets must not change the window's
appearance. Repeat at real 100%, 125%, 150% and 200% DPI when available.

The project does not yet have a deterministic headless Slint screenshot renderer. Do not commit unreviewed images as golden baselines. The CI workflow runs formatting, linting, tests, and release builds on Windows, Linux, and macOS; visual golden-image automation will be added when a stable renderer harness is available.

## Clock Frame Desktop Probe (MODE-06A)

Run `cargo run -p oziclock-desktop --example clock_frame_gallery -- target/clock-frame`
in a graphical desktop session. This uses the production AppWindow with fixed
time, colors, and column data, without loading or saving user settings. It exits
after eleven scenarios and fails on missing edges, opaque outer corners,
unsnapped geometry, lost text area, or incorrect column-drag mapping. FemtoVG
snapshots are taken after rendering and before the buffer swap.

The optional second argument multiplies application scale, for example
`cargo run -p oziclock-desktop --example clock_frame_gallery -- target/clock-frame-125 1.25`.
Repeat at multipliers 1, 1.25, 1.5, and 2. The scenarios additionally cover 80%
and 150% application scales, radius 0/12/15.5, one/three clocks, seconds off,
inactive opacity, and compact/standard/extended modes.

`SLINT_SCALE_FACTOR=2` also exercises high-DPI rendering without changing system
settings. Fractional environment overrides on a 100% Windows monitor can produce
a Winit window at the wrong physical size; the probe deliberately rejects that
capture. Application-scale coverage is not a substitute for testing real 125%
and 150% monitors, mixed-DPI movement, or macOS native opacity/shadows.

Reviewed Windows references are in `tests/golden/clock-frame/windows/`, with
capture details in its README. Review text clearance, joins, corners, and ruler
alignment before replacing a baseline. Pixel assertions supplement that review;
they do not compare font rasterization across operating systems.
