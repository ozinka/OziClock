# Reviewed clock frame references

Captured and visually reviewed on Windows, 2026-09-24, with Slint 1.17.1,
Winit/FemtoVG OpenGL and the local capsule renderer patch. These are real
AppWindow renderer snapshots with fixed time and bundled Carlito fonts, not
mockups. See `docs/VISUAL_TESTING.md` for regeneration and review instructions.

Accepted references were refreshed for quarter-radius outer padding (4 pixels
per end at maximum radius and 100% scale). Edge digits and the complete frame
were visually reviewed again, and the same 55-scenario matrix passed. The
pre-renderer-fix image remains historical and intentionally retains the old padding.

- Unqualified names: `clock_frame_gallery`, display scale 1 and application-scale
  multiplier 1. `small` and `large` select the probe's 80% and 150% UI cases.
- `capsule-ui125.png`: multiplier 1.25, display scale 1.
- `compact-ui150.png`: multiplier 1.5, display scale 1.
- `capsule-dpi200.png`: `SLINT_SCALE_FACTOR=2`, multiplier 1.
- `capsule-before-renderer-fix.png`: original full-radius padding at display scale 1,
  with the original FemtoVG renderer. It is a negative regression reference,
  not an accepted golden: right midpoint alpha is 160 instead of 255.

Reviewed: continuous outline, antialiased transparent corners, single black
joins, preserved seconds clearance, and aligned ruler/lens columns. The regular
and compact modes and capsule at 100% and 200% were inspected directly.
All 55 scenarios passed geometry, alpha, and physical-window-size assertions
(eleven cases at UI multipliers 1/1.25/1.5/2, plus eleven at display scale 2).

These references do not establish macOS native-opacity behavior, real fractional
monitor DPI, cross-monitor movement, or Linux. Do not accept changes solely from
a pixel test or replace these images without visual review.
