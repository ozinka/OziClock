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

## Expanded Timer Panel (TMR-11 / BL-105)

Manual acceptance is pending. Use Planner at its minimum 900 × 780 size and a
larger window, in Light and Dark themes. Check the new expand icon in each saved
timer row, including enough rows to show the scrollbar; title, countdown, Reset,
Start/Pause and Delete must not overlap. Ordinary Start keeps the list visible.

Expand an idle timer, start it, pause/resume and reset; collapse while running
and verify the same remaining time in the list. Expand another timer and verify
both continue. Switch sections or hide/reopen Planner and verify the chosen
expanded timer is retained for the session. The collapse button returns to the
list without changing countdown state. Check long names and hour/day durations
(including 999d 23:59:59) for clipping; resizing should scale the digits while
keeping all controls in the panel. New-session startup returns to the list.
Verify the title is above the state label and digits, with no overlap at either
window size; the first user capture exposed implicit vertical centering of the
title, now corrected with an explicit top position.

Let a short one-shot timer finish: verify red `00:00`, `Time's up`, the usual
sound/attention card, and Restart. Dismiss the attention card and verify the large
view still shows completion; Restart starts the full duration and Reset returns
to Ready. A repeating timer must show its next interval while attention is
pending. No new golden baseline has been approved for this panel; retain the
existing clock-frame references unchanged until user-reviewed captures exist.

## Inline Timer Completion (TMR-12 / BL-106)

Manual acceptance and new reviewed captures are pending. Finish a short timer
while its expanded panel is visible and Planner has focus. Verify the panel's
accent border and glow pulse smoothly, the first completion timestamp is shown,
the usual sound plays, and Dismiss/Restart replace the normal controls. No
compact timer card or duplicate system banner should appear for that delivery.
Check both themes and minimum/larger Planner sizes; titles and controls must
remain separate from the digits and the glow must not obscure content.

Before dismissing, collapse the panel, change section, open an editor, switch to
another application, hide Planner and minimize it in separate trials. The same
pending attention must appear in the compact card; returning to the focused
expanded panel moves it back without another sound/banner. Check Dismiss stops
the pulse/sound and retains one-shot `00:00`; Restart stops the pulse/sound and
starts the full duration. For a repeating timer, verify the next interval
continues during attention and after Dismiss, while Restart begins a new full
sequence. Sound Off remains silent. With two pending timers, the FIFO front
controls the attention surface and neither item is skipped or shown twice.
Restart the application with unacknowledged attention and verify compact-card
recovery, then expand the matching timer to move attention into Planner.

## Clock Frame Desktop Probe (MODE-06A)

Run `cargo run -p oziclock-desktop --example clock_frame_gallery -- target/clock-frame`
in a graphical desktop session. This uses the production AppWindow with fixed
time, colors, and column data, without loading or saving user settings. It exits
after thirteen scenarios and fails on missing edges, opaque outer corners,
unsnapped geometry, lost text area, or incorrect column-drag mapping. FemtoVG
snapshots are taken after rendering and before the buffer swap.

The optional second argument multiplies application scale, for example
`cargo run -p oziclock-desktop --example clock_frame_gallery -- target/clock-frame-125 1.25`.
Repeat at multipliers 1, 1.25, 1.5, and 2. The scenarios additionally cover 80%
and 150% application scales, radius 0/12/15.5, one/three clocks, seconds off,
inactive opacity, and compact/standard/extended modes. The `capsule-extended`
and `single-extended` cases cover centered ruler/lens labels at maximum radius
with three columns and a single column padded at both ends.

`SLINT_SCALE_FACTOR=2` also exercises high-DPI rendering without changing system
settings. Environment overrides on a 100% Windows monitor can produce a Winit
window at the wrong physical size; the probe deliberately rejects that capture.
This was previously observed with fractional overrides and also reproduced with
override 2 on 2026-09-26 (native size 304 x 62 instead of 608 x 124). Retain the
assertion and report that display-scale run as unverified. Application-scale
coverage is not a substitute for testing real 125%
and 150% monitors, mixed-DPI movement, or macOS native opacity/shadows.

Reviewed Windows references are in `tests/golden/clock-frame/windows/`, with
capture details in its README. Review text clearance, joins, corners, and ruler
alignment before replacing a baseline. Pixel assertions supplement that review;
they do not compare font rasterization across operating systems.
