# OziClock Product and Architecture Notes

The detailed, implementation-derived behavior is maintained in [REQUIREMENTS.md](REQUIREMENTS.md). Exact ruler and magnifying-lens behavior is specified in [RULER_LENS_DESIGN.md](RULER_LENS_DESIGN.md).

The target module boundaries and dependency rules are defined in [ARCHITECTURE.md](ARCHITECTURE.md). Candidate alarms, reminders, timers, and stopwatch capabilities are scoped in [FUTURE_FEATURES.md](FUTURE_FEATURES.md).

## Product Intent

OziClock is a compact desktop world clock for people who work across time zones. It normally floats above other windows and prioritizes information density, precise alignment, and immediate visual comparison. The existing WPF application is the behavioral and visual reference for a future cross-platform rewrite.

The screenshots in `legacy/dotnet-wpf/Ozi.Clock/Assets/` are reference snapshots for the three principal display modes:

- `ozi.clock.small.webp`: time-only clock tiles.
- `ozi.clock.mid.webp`: clock tiles with zone label and date.
- `ozi.clock.large.webp`: clock tiles, synchronized vertical rulers, and a time-shift slider.

## Core UI Modules

### Clock Tile

A tile represents one configured time zone. Tiles have a fixed reference size of 99 × 60 logical pixels. The strip itself is black, with one logical pixel of outer padding and one logical pixel between tiles; tiles have no individual black border. This preserves a single-pixel divider instead of doubling it where tiles meet. Each tile contains:

- a short user-defined label;
- month/day and 24-hour time;
- optional seconds, with the remaining digits recentered when seconds are hidden;
- a configurable accent color;
- a vertical gradient from dark gray (`#383838`) to the accent color.

The selected main zone uses stronger white label text; other labels are subdued. Font metrics, baselines, margins, dividers, and gradient stops are product behavior, not incidental styling. Visual-regression snapshots should protect them during migration.

The legacy reference geometry is retained: label `x=4,y=5,w=48`, month `x=51,w=28`, day `x=80,w=18` (all 16 px); hour `x=0,y=35,w=28`, minute `x=34,y=35,w=38` (22 px), and seconds `x=67,y=41,w=38` (16 px). Without seconds, hour/minute shift right by 14 px. Use bundled Carlito, a metric-compatible Calibri alternative, a dark-to-accent vertical gradient, and an unframed draggable window.

### Clock Strip and Context Menu

The main window is frameless, compact, draggable, optionally absent from the taskbar, and normally always on top. It composes any number of tiles horizontally. Its context menu supports adding, editing, reordering or removing clocks, choosing the main zone, toggling seconds and display modes, opening settings, and exiting. Configuration and window position persist between launches.

### Time Rulers and Slider

Each clock can extend into a vertical ruler showing the progression of local time. All rulers share one horizontal focus band so equivalent moments line up across zones. The main-zone ruler is emphasized. A slider shifts the common reference time in small increments, allowing future or past comparisons while preserving daylight-saving and non-whole-hour offsets. Ruler width, focus position, and vertical magnification are adjustable in the legacy implementation.

## Domain Boundaries

Keep the rewrite split into independently testable layers:

1. **Domain:** clock configuration, main-zone selection, instant shifting, and local-time conversion.
2. **Persistence:** versioned user settings and migration from legacy JSON.
3. **Presentation state:** display mode, tile ordering, ruler viewport, and menu commands.
4. **Rendering:** clock tile, clock strip, rulers, slider, dialogs, and color picker.
5. **Platform services:** always-on-top, frameless dragging, taskbar/dock and tray behavior, startup, and window placement.

Store instants in UTC and render them through IANA time-zone identifiers. The legacy configuration uses Windows IDs such as `FLE Standard Time`; import must map these to IANA equivalents and retain an explicit fallback for unknown zones.

## Migration Strategy

The WPF reference is preserved in `legacy/dotnet-wpf/`; its solution, release script, source, assets, and README move together. The active rewrite starts in `apps/oziclock-desktop/`, with shared golden images under `tests/golden/`. Do not delete the legacy source until the cross-platform application reproduces the small mode and the release prototype has passed its validation gate.

Recommended delivery slices are: domain/time-zone tests; pixel-matched clock tile; multi-tile window and persistence; context menu/editing; rulers and shifted time; then platform packaging and release automation.
