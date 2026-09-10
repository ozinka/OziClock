# ADR 0004: Observe macOS Window State Without Changing Window Style

- Status: Accepted
- Date: 2026-09-10
- Requirements: WIN-01, WIN-03, MODE-04; backlog BL-013

## Context

Slint 1.17.1 queries winit's maximized state on resize and occlusion events.
In winit 0.30.13, that getter temporarily adds Titled and Resizable style
bits. AppKit constrains the taller frame under the menu bar; removing the
style leaves a borderless window displaced. Isolated native and Slint probes
reproduced a 32-point displacement on startup and resize. Additional
borderless attributes and changing the resize API do not prevent this getter.

## Decision

Pin the winit 0.30.13 source distribution in `vendor/winit` with a root Cargo
patch. Remove style mutation from macOS maximization observation. Use native
`isZoomed` for titled resizable windows. For other windows, compare the actual
frame with the current screen's visible frame, reporting false while minimized.
Use that same visible frame for explicit maximization and the saved standard
frame for restoration. This keeps geometry changes in commands, not getters,
and allows a later move or resize to be reflected without stale cached state.

Keep all other platforms unchanged. Do not introduce application position
correction timers, event suppression, native method swizzling or screen
constraint overrides. Do not replace Slint or change the clock rendering.

## Consequences and Validation

The repository carries approximately 3 MB of dependency source and must review
this patch on upgrades. `vendor/README.md` records provenance, scope and removal
criteria. The native `macos_window_geometry` example exercises the actual
patched dependency through Slint's event loop, including startup at the work-area
top, repeated mode-height/scale changes, query invariance, minimize/restore,
and borderless/decorated maximize/restore. It requires a graphical session and
is not part of headless unit tests. Workspace checks remain mandatory.

Actual logout/login, Retina/multi-monitor changes and full-screen Spaces
transitions require separate platform verification; they are not established
by compilation or the automated geometry probe. Keep BL-013 open while those
acceptance checks remain outstanding.
