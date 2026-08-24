# OziClock Requirements

This catalog is derived from the WPF source, its bundled screenshots, and the intended cross-platform migration. “Legacy” means the behavior exists in the current code. “New” means it is required for the rewrite but is not implemented in WPF.

## Application Window

- **WIN-01 (Legacy):** Present the clock strip in a frameless, non-resizable window with a transparent outer surface and no standard title bar.
- **WIN-02 (Legacy):** Allow dragging the strip by holding the left mouse button anywhere on its background.
- **WIN-03 (Legacy):** Persist and restore the main window position.
- **WIN-04 (Legacy):** Offer an “Always on top” setting. When enabled, keep the strip above other windows without stealing focus.
- **WIN-05 (Legacy):** Allow the main window to be shown in or hidden from the operating-system taskbar/dock.
- **WIN-06 (Legacy):** Support configurable inactive opacity from 2% to 100%. Animate to 100% over 300 ms while hovered or focused and return to the configured opacity when inactive.
- **WIN-07 (New):** Provide a system-tray/status-item icon whenever the taskbar/dock entry is hidden. Its menu must at minimum contain Show/Hide, Settings, Always on top, and Exit. Closing or hiding the window must not make the app unreachable.
- **WIN-08 (New):** Restore an off-screen saved position into the current working area after monitor or resolution changes.

## Display Modes

- **MODE-01 (Legacy):** Standard mode displays the complete 99 × 60 logical-pixel tile: zone label and month/day on top, time below.
- **MODE-02 (Legacy):** Compact mode folds the strip to approximately 29 logical pixels high, hiding the date/header area and leaving the time visible. Switching is available through `Fold/Unfold` and middle-click and uses a 200 ms animation.
- **MODE-03 (Legacy):** Extended mode attaches the ruler window and time slider below the strip. Switching is available through `Show/Hide Rulers` and double-click.
- **MODE-04:** Mode changes must preserve tile order, selected main zone, window position, colors, and current settings.
- **MODE-05 (New):** Persist the selected display mode across restarts.

## Clock Collection and Tile

- **CLK-01 (Legacy):** Display one or more clocks in a horizontal strip. Each clock represents one configured time zone.
- **CLK-02 (Legacy):** A tile contains a short editable label, month/day, 24-hour hours and minutes, and optional seconds.
- **CLK-03 (Legacy):** Recenter hours and minutes when seconds are hidden.
- **CLK-04 (Legacy):** Update live time once per second in normal modes.
- **CLK-05 (Legacy):** Give every tile a configurable pastel accent color and render a vertical dark-gray-to-accent gradient with pixel-aligned boundaries.
- **CLK-06 (Legacy):** Exactly one clock is the main zone. Emphasize its label in white and use it as the reference for rulers and shifted time.
- **CLK-07 (Legacy):** Add a new clock initially as UTC, then immediately open its editor.
- **CLK-08 (Legacy):** Edit the selected clock’s label, system time zone, and accent color with immediate preview.
- **CLK-09 (Legacy):** Move a clock one position left or right and keep its ruler in the same order.
- **CLK-10 (Legacy):** Remove a non-main clock only after confirmation. Never remove the last clock or the main clock.
- **CLK-11:** Persist label, portable time-zone identifier, color, order, main selection, and seconds preference in the per-user JSON settings file. The UI must not hardcode a clock list.
- **CLK-12:** Convert a UTC instant correctly through daylight-saving transitions and time zones with 30- or 45-minute offsets.

## Context Menu

Right-clicking a tile must target that tile and show its label as a disabled menu heading.

| Item | Required behavior | Availability |
| --- | --- | --- |
| Edit | Open the targeted clock editor | Always |
| Move Left | Shift clock and ruler left by one | Hidden for first clock |
| Move Right | Shift clock and ruler right by one | Hidden for last clock |
| Make Main | Select the reference clock | Hidden for current main clock |
| Remove | Confirm and delete targeted clock | Hidden for main or only clock |
| Add Clock | Append UTC clock and open editor | Always |
| Fold / Unfold | Toggle compact and standard modes | Label reflects current mode |
| Show / Hide Rulers | Toggle extended mode | Label reflects ruler visibility |
| Settings | Open application settings | Always |
| About | Show version, capabilities, project link, credits, and license | Always |
| Exit | Save settings and terminate all windows | Always |

## Rulers and Time Exploration

Exact geometry, visual effects, Slint layering, and renderer acceptance criteria are defined in [RULER_LENS_DESIGN.md](RULER_LENS_DESIGN.md).

- **RUL-01 (Legacy):** Attach one 99-pixel-wide vertical ruler beneath each tile and keep all auxiliary windows aligned when the main strip moves or changes height.
- **RUL-02 (Legacy):** Draw a 24-hour scale with minor ticks on both sides and 25 labels, including fractional offsets such as `12:30`.
- **RUL-03 (Legacy):** Align every ruler against the selected main zone and emphasize the main ruler with a bright focus column and red edges.
- **RUL-04 (Legacy):** Present a shared horizontal focus band across all rulers with shaded/blurred regions outside it.
- **RUL-05 (Legacy):** Allow dragging the horizontal focus band; synchronize its position with the slider.
- **RUL-06 (Legacy):** Allow horizontal movement of the main-zone focus column within the clock strip’s bounds.
- **RUL-07 (Legacy):** Provide a 0–24-hour slider with five-minute resolution (`0…288`). Round the initial reference time to the nearest hour when extended mode opens.
- **RUL-08 (Legacy):** Scale slider width and label density with clock count and update clocks interactively while the slider moves.

## Settings and Persistence

- **SET-01 (Legacy):** Settings include opacity, show in taskbar, always on top, and show seconds.
- **SET-02:** Apply all settings immediately and consistently to every visible window; no restart may be required.
- **SET-03:** Save settings atomically in `settings.json` beside the executable and recover with safe defaults from missing, invalid, or older configuration.
- **SET-04:** Version the settings schema and import legacy Windows time-zone IDs into IANA IDs.
- **SET-05 (New):** Persist display mode, ruler/focus position where appropriate, and tray/window visibility.

## Auxiliary Windows and Interaction

- **UI-01 (Legacy):** Edit, Settings, About, color picker, rulers, and slider use frameless custom styling and do not create separate taskbar entries.
- **UI-02 (Legacy):** Position Edit and Settings near the clock strip, preferring below it, falling back above, and constraining them to the working area.
- **UI-03 (Legacy):** The color picker offers the existing curated palette, closes after selection, and dismisses when it loses focus.
- **UI-04:** Dialogs must remain reachable by keyboard; Escape cancels where safe, Enter accepts, and focus indication must be visible.
- **UI-05:** Context-menu and mouse shortcuts must have discoverable menu equivalents.

## Cross-Platform Quality Requirements

- **NFR-01:** Support current Windows, macOS, and mainstream Linux desktop releases.
- **NFR-02:** Preserve proportions, baselines, one-pixel edges, gradients, and seamless tile joins at 100%, 125%, 150%, and 200% scale.
- **NFR-03:** Use golden-image tests for the three reference modes and unit tests for time conversion, DST boundaries, ordering, removal rules, and persistence migration.
- **NFR-04:** Keep idle CPU use near zero apart from the once-per-second clock update; extended interactive mode may update more frequently only while visible.
- **NFR-05:** Work without network access and collect no telemetry by default.
- **NFR-06:** Use platform-appropriate context menus, tray/status items, startup behavior, packaging, and signing without leaking platform APIs into domain logic.

## Legacy Gaps to Avoid

The WPF source has no tray icon despite supporting `ShowInTaskbar = false`. Its topmost code contains an unresolved TODO, and disabling the setting does not explicitly demote an already-topmost window. Settings objects do not notify all bound views, so taskbar visibility and seconds may not update reliably at runtime. The rewrite must implement the requirements above rather than reproduce these limitations.
