# ADR 0005: Let Slint Own the Clock Frame and Fix Capsule Rendering in the Renderer

- Status: Accepted
- Date: 2026-09-24
- Requirements: MODE-06, MODE-06A, WIN-15; backlog BL-087

## Context

The clock relied on background gaps for its outline. Its final tile occupied the
right-hand gap, and compact translation hid the original top gap. Radius-sized
outer padding is also needed to keep rounded ends out of the existing text area.

An explicit Slint border fixes this geometry, but the stock FemtoVG renderer in
Slint 1.17.1 still leaves a translucent notch at the right midpoint of a compact
capsule. The deterministic AppWindow probe reproduced this with one and three
clocks at radius 15.5. A border-colored background improved alpha from 64 to 160,
but did not fix it. Slint already special-cases circles to avoid the same
zero-length straight edges between adjacent Bezier curves.

## Decision

Use one `ClockFrame` with Slint border and rounded child clipping. Share snapped
column metrics and boundary mapping between clocks, rulers, lens, and native
window sizing. Add padding only at the two outer ends: one quarter of the scaled
radius, rounded up to a physical pixel, following the user's Windows visual review.
Do not calculate separate
inner radii, layer edge masks, or reduce the requested maximum radius.

Carry a narrowly patched Slint FemtoVG renderer alongside the existing patched
Winit dependency. Extend its circle handling to capsules, omitting zero-length
straight segments and keeping corner radii circular when they reach half the
shorter side. Other renderer behavior is unchanged. The UI still uses the
standard Rectangle border API. `vendor/README.md` records provenance and removal.

## Alternatives and Consequences

The software renderer failed this probe's border/transparency checks. Skia was
built for investigation with dynamic CRT, but the matching static-CRT prebuilt
archive returned 404; source compilation required additional LLVM tools.
Neither experiment changed the production renderer or packaging. A renderer
switch would need broader compatibility and resource measurements.

The renderer source copy adds roughly 185 KB and must be reviewed on dependency
upgrades. The patch adds no additional UI layers, timers, or per-frame work beyond
constructing capsule paths. The desktop gallery captures the actual AppWindow,
checks physical geometry and border alpha, and supplies reviewed visual references.
macOS and real fractional-DPI monitor transitions remain platform validation
gates, not outcomes established by compilation or a Windows screenshot probe.
