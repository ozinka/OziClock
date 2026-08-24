# Ruler and Lens Design Specification

## Purpose

Extended mode visualizes the same instant across all configured time zones. Each 99-pixel clock tile continues downward as a vertical 24-hour ruler. A movable vertical focus selects one clock-sized column, while a movable horizontal lens selects and magnifies one moment across every column. Together they make offsets, including fractional-hour offsets, immediately comparable.

For future discussion and code labels, call the horizontal time-selection strip the **Lens** and the vertical column focus the **Column slider**. Either control moves only after it is clicked or dragged: the Lens moves vertically and the Column slider moves horizontally. At their intersection, the Lens has priority.

The WPF implementation and `legacy/dotnet-wpf/Ozi.Clock/Assets/ozi.clock.large.webp` are the visual references. The rewrite should preserve the visual intent while avoiding dependence on WPF `VisualBrush` behavior.

## Current Visual Reference

`legacy/dotnet-wpf/Ozi.Clock/Assets/ozi.clock.large.webp` is useful historical evidence, but the user-provided screenshots from 2026-08-24 are the current visual reference. In particular, they confirm that the ruler strip and the time slider remain separate frameless windows attached below the clock strip.

## Legacy Geometry and Effects

- Each ruler column is **99 logical pixels** wide with a one-pixel join.
- The ruler area is **463 logical pixels** high.
- The horizontal lens is **30 logical pixels** high.
- Its vertical focus position moves through `0…433` logical pixels.
- Content inside the horizontal lens is magnified **1.6× vertically**; it is not enlarged horizontally.
- The legacy transform also applies a relative vertical translation of `-0.25`.
- Non-focused regions use opacity `0.5` and WPF blur radius `4`.
- The focused vertical column is bounded by dark-red two-pixel edges.
- The vertical focus is initially aligned with the selected main clock and may be dragged horizontally within the strip.
- The ruler surface has a one-pixel black outer border and one-pixel black joins between columns.
- Every column uses the clock's gradient/color background; it is not a shared neutral background.

### Ruler Content Geometry

- A column is 99 pixels wide. The first tick starts at Y=15; six ticks per hour are spaced 3 pixels apart, using lengths `25, 15, 15, 20, 15, 15` on both left and right edges.
- There are 25 labels, beginning at Y=6 with an 18-pixel step. Labels are centered in the 99-pixel column.
- The main clock labels `0…24`, including `24` only at the final label. Every other clock applies its current offset and wraps after `23` to `0`; fractional offsets render labels such as `4:30` and `5:45`.
- The horizontal slider is 69 pixels high, has a black one-pixel outer edge, and a `#383838` inner surface. Its width is `clock_count × 100 + 1` pixels and it uses 0…288 five-minute steps.

The horizontal lens uses this edge-darkening overlay:

| Position | Color |
| ---: | --- |
| 0% | `#88000000` |
| 10% | `#55000000` |
| 40% | `#00000000` |
| 60% | `#00000000` |
| 90% | `#55000000` |
| 100% | `#88000000` |

The transparent center preserves sharp labels; the shaded upper and lower edges create the appearance of a horizontal glass rod.

## Interaction Requirements

- Dragging the vertical focus moves the 99-pixel highlighted column without changing clock order.
- The Column slider moves freely, but snaps to a ruler-column boundary when it comes within five logical pixels of that boundary.
- Dragging the horizontal lens continuously updates its vertical position.
- Lens position and the 0–24-hour slider remain synchronized in both directions.
- The slider has five-minute resolution (`0…288`) and maps linearly to the lens range.
- All columns show the same UTC instant converted into their own zones. Labels must handle whole-hour, half-hour, and 45-minute offsets.
- Movement must remain bounded; neither focus may leave the ruler surface.
- Pointer capture must continue a drag when the pointer temporarily leaves the handle and release cleanly on pointer-up or cancellation.
- The ruler window is attached directly below the clock strip; the slider window is attached directly below the rulers. Moving the clock strip, changing its scale, changing compact/standard mode, or changing clock count repositions and resizes both attached windows as one visual unit.

## Slint Rendering Design

Use one immutable ruler data model as the source for two presentations:

1. the normal full-height ruler strip;
2. a second ruler strip clipped to the 30-pixel horizontal lens and scaled `1.6` on the Y axis.

Do not take bitmap snapshots of the main UI or emulate WPF `VisualBrush`. Re-rendering the same small model keeps text and ticks sharp at different DPI scales and makes golden-image testing deterministic.

The Slint scene should be layered as follows:

```text
RulerPanel
├── NormalRulerStrip
├── LeftDimmer
├── VerticalFocus (99 px)
├── RightDimmer
└── HorizontalLens (30 px, clipped)
    ├── MagnifiedRulerStrip (Y scale 1.6)
    └── EdgeDarkeningGradient
```

Implement dimensions and colors as named design tokens rather than scattering constants through components. Calculate tick and label data in Rust; keep positioning, clipping, gradients, and pointer handling in Slint.

The desktop adapter owns three native windows (`AppWindow`, `RulersWindow`, and `TimeSliderWindow`) and one placement coordinator. The coordinator is the only code allowed to position or resize the two attached windows; it receives main-window movement, mode, scale, and clock-count changes. The ruler and slider components exchange a typed selected-time value rather than manipulating each other's widgets.

## Renderer Decision

Prototype with the Slint **Winit + FemtoVG** renderer. It supports transforms required for vertical magnification and should provide smooth GPU-accelerated dragging. Skia is the fallback if text quality or platform compatibility is better in testing.

Slint’s software renderer currently does not support item scale transforms. A later software-renderer experiment may draw the lens with explicitly multiplied Y coordinates and font sizes, but it must only replace FemtoVG if measurements show a material resource benefit without visual regressions.

Arbitrary background blur is not a portability requirement. First reproduce the perceived effect with semi-transparent dimmers and the specified gradient. Add renderer-specific blur only if side-by-side screenshots show that it materially improves the design.

## Performance and Visual Acceptance

- Maintain **60 FPS** while either focus is dragged on supported hardware.
- Return to event-driven idle rendering immediately after interaction.
- Update normal clock time only once per second; do not retain the WPF 20 ms polling loop.
- Produce no visible seams, clipping errors, or baseline jumps at 100%, 125%, 150%, and 200% display scaling.
- Preserve one-pixel ticks and borders where physically possible through pixel alignment.
- Add golden images for top, center, and bottom lens positions, a non-main vertical focus, and a half-hour-offset zone.
- Benchmark release builds for working-set memory, idle CPU, drag CPU/GPU use, startup time, and renderer-specific package size before finalizing the renderer.
