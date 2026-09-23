# Patched dependencies

## Slint FemtoVG renderer

`i-slint-renderer-femtovg/` is the crates.io Slint 1.17.1 renderer source
distribution. Its upstream revision is in `.cargo_vcs_info.json`; licensing
is in its `LICENSES/` directory and `Cargo.toml`. Registry cache markers and
the dependency's lock file are omitted. The root Cargo patch selects this copy.

OziClock patch (BL-087, MODE-06A, WIN-15): only `itemrenderer.rs`, in
`rect_with_radius_to_path`. Extend the existing circle special case to capsules:
clamp the uniform radius to half the shorter side, and connect the quarter-circle
curves without zero-length straight edges. The unpatched maximum-radius compact
clock has alpha 160 instead of 255 at the middle of its right border at 100% DPI.
The patched renderer passes the same pixel check without changing the UI radius,
border thickness, renderer selection, or Windows static-CRT packaging.

Regression: `cargo run -p oziclock-desktop --example clock_frame_gallery` in a
graphical session; see `docs/VISUAL_TESTING.md` and ADR 0005. Keep this patch small
and compare with upstream on Slint upgrades. Remove the override and directory
when an upstream version passes the capsule regression without this patch.

## Winit

`winit/` is the crates.io winit 0.30.13 source distribution (Apache-2.0;
see `winit/LICENSE`). Cargo selects it through the root `[patch.crates-io]`.
Its `.cargo_vcs_info.json` identifies the upstream revision. Registry cache
markers/checksums are omitted because this is an intentionally modified path dependency.

OziClock patch (BL-013, WIN-01, WIN-03, MODE-04):
`src/platform_impl/macos/window_delegate.rs` no longer temporarily changes
NSWindow style when querying maximization. Titled resizable windows retain
AppKit's native zoom behavior. Borderless/non-resizable windows use their
actual frame compared with the current screen's visible frame; explicit
maximize/restore uses the same geometry and the saved standard frame.
Windows, Linux and other platform implementations are unchanged.

Regression: `cargo run -p oziclock-desktop --example macos_window_geometry`
in a macOS graphical login session. Also perform the normal workspace checks.
The example checks native frame/style invariance, top-edge startup/resize,
85%/100% scale sizes, and borderless/decorated maximize/restore. It does not
simulate logout/login, Retina display changes or full-screen Spaces transitions.

Keep the source patch explicit and small. Do not edit the global Cargo cache.
On upgrade, compare this file with upstream and rerun the native probe before
removing the patch. Remove this directory and the Cargo override once the
selected upstream release provides equivalent behavior. Do not add geometry
correction timers or bypass AppKit's work-area constraints.
