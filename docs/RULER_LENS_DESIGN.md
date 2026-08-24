# Ruler and Lens Design Specification

## Purpose

Extended mode visualizes the same instant across all configured time zones. Each 99-pixel clock tile continues downward as a vertical 24-hour ruler. A movable vertical focus selects one clock-sized column, while a movable horizontal lens selects and magnifies one moment across every column. Together they make offsets, including fractional-hour offsets, immediately comparable.

The WPF implementation and `legacy/dotnet-wpf/Ozi.Clock/Assets/ozi.clock.large.webp` are the visual references. The rewrite should preserve the visual intent while avoiding dependence on WPF `VisualBrush` behavior.

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
- Dragging the horizontal lens continuously updates its vertical position.
- Lens position and the 0–24-hour slider remain synchronized in both directions.
- The slider has five-minute resolution (`0…288`) and maps linearly to the lens range.
- All columns show the same UTC instant converted into their own zones. Labels must handle whole-hour, half-hour, and 45-minute offsets.
- Movement must remain bounded; neither focus may leave the ruler surface.
- Pointer capture must continue a drag when the pointer temporarily leaves the handle and release cleanly on pointer-up or cancellation.

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
