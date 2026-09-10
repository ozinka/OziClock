# Patched dependencies

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
