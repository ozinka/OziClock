# Reviewed clock frame references

Captured and visually reviewed on Windows, 2026-09-26, with Slint 1.17.1,
Winit/FemtoVG OpenGL and the local capsule renderer patch. These are real
AppWindow renderer snapshots with fixed time and bundled Carlito fonts, not
mockups. See `docs/VISUAL_TESTING.md` for regeneration and review instructions.

Current references use eighth-radius outer padding (2 pixels per end at maximum
radius and 100% scale). Normal ruler and lens labels center across the full
padded columns. Edge digits, the complete frame and label symmetry were visually
reviewed again. The pre-renderer-fix and DPI-200 images remain historical and
intentionally retain their earlier padding.

- Unqualified names: `clock_frame_gallery`, display scale 1 and application-scale
  multiplier 1. `small` and `large` select the probe's 80% and 150% UI cases.
- `capsule-ui125.png`: multiplier 1.25, display scale 1.
- `compact-ui150.png`: multiplier 1.5, display scale 1.
- `capsule-extended.png` and `single-extended.png`: maximum radius with three/one
  columns, including full-width centered ruler and lens labels.
- `capsule-dpi200.png`: historical 2026-09-24 quarter-radius reference,
  `SLINT_SCALE_FACTOR=2`, multiplier 1; not a baseline for the current padding.
- `capsule-before-renderer-fix.png`: original full-radius padding at display scale 1,
  with the original FemtoVG renderer. It is a negative regression reference,
  not an accepted golden: right midpoint alpha is 160 instead of 255.

Reviewed: continuous outline, antialiased transparent corners, single black
joins, preserved seconds clearance, and aligned ruler/lens columns. The regular
and compact modes, three/single-column rulers and the maximum-radius extended
case at UI scale 200% were inspected directly. All 52 scenarios passed geometry,
alpha, and physical-window-size assertions (thirteen cases at UI multipliers
1/1.25/1.5/2, display scale 1). The display-scale override 2 run failed twice at
the initial native-size assertion (304 x 62 instead of 608 x 124), so its previous
reference was not replaced and the current DPI-200 rendering remains unverified.

These references do not establish macOS native-opacity behavior, real fractional
monitor DPI, cross-monitor movement, or Linux. Do not accept changes solely from
a pixel test or replace these images without visual review.
