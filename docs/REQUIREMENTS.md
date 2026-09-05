# OziClock Requirements

This catalog is derived from the WPF source, its bundled screenshots, and the intended cross-platform migration. “Legacy” means the behavior exists in the current code. “New” means it is required for the rewrite but is not implemented in WPF.

## Application Window

- **WIN-01 (Legacy):** Present the clock strip in a frameless, non-resizable window with a transparent outer surface and no standard title bar.
- **WIN-02 (Legacy):** Allow dragging the strip by holding the left mouse button anywhere on its background.
- **WIN-03 (Legacy):** Persist and restore the main window position.
- **WIN-04 (Legacy):** Offer an “Always on top” setting. When enabled, keep the strip above other windows without stealing focus.
- **WIN-05 (Legacy):** Allow the main window to be shown in or hidden from the operating-system taskbar/dock.
- **WIN-06 (Legacy):** Support configurable inactive opacity from 2% to 100% in General settings. Animate to 100% over 300 ms while hovered or focused and return to the configured opacity when inactive.
- **WIN-06A:** On macOS, apply the clock's opacity to the native window, including its system shadow. Keep the Slint content opaque to avoid multiplying opacity. Preserve the outline and one-pixel clock separators. Native shadow fading requires desktop visual verification.
- **WIN-07 (New):** Provide a system-tray/status-item icon whenever the taskbar/dock entry is hidden. Its menu must at minimum contain Show/Hide, Settings, Always on top, and Exit. Closing or hiding the window must not make the app unreachable.
- **WIN-08 (New):** Restore an off-screen saved position into the current working area after monitor or resolution changes.
- **WIN-10:** Keep the context menu entirely within the active monitor's working area; open it above the clock strip when there is insufficient space below.
- **WIN-11:** Let users set a persisted clock UI scale from 80% to 150% without changing the operating-system display scale.
- **WIN-12:** Open Settings near the clock while keeping the window entirely within the active monitor's working area.
- **WIN-13:** Restore the last Settings window size at application startup and persist a changed size when Settings is saved, closed, or the application exits.
- **WIN-14:** Resize the clock strip immediately after adding or removing a clock, leaving no unused background area.
- **WIN-15:** Provide a persisted outer-corner radius from 0 to 15.5 logical pixels. The same logical radius applies to the clock in standard and compact modes and to Calendar, Settings, and About forms; the maximum produces semicircular compact-mode ends.
- **WIN-16:** Let users choose one persisted color for the outer outline and the one-pixel separators between clock, ruler, and slider blocks.

## Display Modes

- **MODE-01 (Legacy):** Standard mode displays the complete 99 × 60 logical-pixel tile: zone label and month/day on top, time below.
- **MODE-02 (Legacy):** Compact mode folds the strip to approximately 29 logical pixels high, hiding the date/header area and leaving the time visible. Switching is available through General settings and middle-click and uses a 200 ms animation. During the transition, full-height tiles remain bottom-anchored and are clipped by the shrinking window.
- **MODE-03:** The clock strip, rulers, and time slider share one native window and one clipped viewport. Compact/standard clock height and ruler visibility are independent states: middle-click changes only clock height, while `Show/Hide Rulers` in the context menu changes only ruler visibility. The viewport resizes to reveal the resulting portion of a single composed surface, including compact clocks with visible rulers.
- **MODE-03A:** The clock strip remains fully opaque while rulers are visible, regardless of the configured inactive opacity.
- **MODE-04:** Mode changes must preserve tile order, selected main zone, window position, colors, and current settings.
- **MODE-05 (New):** Persist the selected display mode across restarts.
- **MODE-06:** One rounded one-pixel outline applies to the outside of the currently visible viewport. Compact and standard modes outline the clock strip; extended mode outlines the complete clock, ruler, and time-slider construction, while both internal joins remain square.

## Clock Collection and Tile

- **CLK-01 (Legacy):** Display one or more clocks in a horizontal strip. Each clock represents one configured time zone.
- **CLK-02 (Legacy):** A tile contains a short editable label, month/day, 24-hour hours and minutes, and optional seconds.
- **CLK-03 (Legacy):** Recenter hours and minutes when seconds are hidden.
- **CLK-04 (Legacy):** Update live time once per second in normal modes.
- **CLK-05 (Legacy):** Give every tile a configurable pastel accent color and render a vertical dark-gray-to-accent gradient with pixel-aligned boundaries.
- **CLK-06 (Legacy):** Exactly one clock is the main zone. Emphasize its label in white and use it as the reference for rulers and shifted time.
- **CLK-07 (Legacy):** Add a new clock initially as UTC, then immediately open its editor.
- **CLK-08 (Legacy):** Edit the selected clock’s label, time zone from the supported IANA time-zone list with its current UTC offset (sorted by offset, then name), and accent color with immediate preview.
- **CLK-08A:** The clock editor provides a time-zone search field above the picker. Filtering is case-insensitive and matches both the IANA identifier and the visible offset-and-name text. Results preserve the full list's current-offset-then-identifier ordering; an empty query restores the complete list. When no time zone matches, the picker is empty and the editor shows a non-blocking `No matching time zones` message. The search field and filtered picker remain operable with the keyboard, and selecting a result updates the edited clock immediately without changing any other clock property.
- **CLK-09 (Legacy):** Reorder a clock by dragging its six-dot handle in the Settings list, immediately keeping the clock strip and its ruler in the same order.
- **CLK-10 (Legacy):** The trailing trash icon in the Settings list opens a confirmation before removing a non-main clock. Never remove the last clock or the main clock.
- **CLK-11:** Persist label, portable time-zone identifier, color, order, main selection, and seconds preference in the per-user JSON settings file. The UI must not hardcode a clock list.
- **CLK-12:** Convert a UTC instant correctly through daylight-saving transitions and time zones with 30- or 45-minute offsets.
- **CLK-13:** Offer the legacy dark-to-accent clock surface and an optional soft color style with no dark upper region, compact header typography, and larger primary time numerals.
- **CLK-14:** Provide persisted dimming from 0% to 80% for non-primary clock tiles. Hovering anywhere over the clock strip smoothly removes dimming from all tiles and smoothly restores it when the pointer leaves.

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
- **RUL-03:** Only the main-clock ruler displays `24` at its final label; all other rulers wrap from `23` to `0` after applying their current time-zone offset.
- **RUL-04:** The ruler surface has a one-pixel black outer border and one-pixel black joins between 99-pixel clock columns.
- **RUL-03 (Legacy):** Align every ruler against the selected main zone and emphasize the main ruler with a bright focus column and red edges.
- **RUL-04 (Legacy):** Present a shared horizontal focus band across all rulers with shaded/blurred regions outside it.
- **RUL-05 (Legacy):** Allow dragging the horizontal focus band; synchronize its position with the slider.
- **RUL-06 (Legacy):** Allow horizontal movement of the main-zone focus column within the clock strip’s bounds.
- **RUL-07 (Legacy):** Provide a 0–24-hour slider with five-minute resolution (`0…288`). Round the initial reference time to the nearest hour when extended mode opens.
- **RUL-08 (Legacy):** Scale slider width and label density with clock count and update clocks interactively while the slider moves.

## Settings and Persistence

- **SET-01 (Legacy):** Settings include opacity, show in taskbar, launch at login, always on top, show seconds, outer-corner radius, outline/separator color, non-primary clock dimming, and clock surface style.
- **SET-02:** Apply all settings immediately and consistently to every visible window; no restart may be required.
- **SET-03:** Save settings atomically in `settings.json` beside the executable on Windows and Linux, and under `~/Library/Application Support/OziClock` on macOS. Recover with safe defaults from missing, invalid, or older configuration, and migrate a legacy macOS settings file found beside `OziClock.app`.
- **SET-04:** Version the settings schema and import legacy Windows time-zone IDs into IANA IDs.
- **SET-05 (New):** Persist display mode, ruler/focus position where appropriate, and tray/window visibility.
- **SET-06:** The settings window separates application-wide controls under General from per-clock controls under Clocks; a clock editor is visible only after selecting that clock.
- **SET-07:** The Clocks list ends with an in-list `Add clock` action rather than a separate footer action.

## Auxiliary Windows and Interaction

- **ALM-09:** Clicking an alarm card selects it without entering edit mode and reveals Edit in the lower action slot. Clicking Edit loads the alarm into the form and changes that action to Cancel. Cancel exits editing but keeps the alarm selected so Edit remains available. Clicking the selected card again deselects it and exits editing. The accent border indicates selection, not merely editing; On/Off and Delete do not select the card.

- **ALM-08:** Override ALM-07's fixed left column: its minimum is 354px and it alone absorbs additional width. Weekday hit targets are 38px wide with 2px spacing, followed immediately by the fixed Once/Weekly control. H:M and the 125px action column remain fixed-width and move right as the window expands. The edited-card accent border is one pixel.

- **ALM-07:** The alarm editor uses a fixed 403px left column so Once/Weekly follows Sun directly. The H:M editor remains in the middle. Add alarm/Save changes occupies the top of the flexible right column, with Cancel directly below only while editing. The edited saved-alarm card is identified by a two-pixel accent border.

- **ALM-06:** Alarm cards open editing in the existing footer with Save changes and Cancel editing. Saving preserves identity, enabled state and saved zone, and recalculates Once's next date. A trailing shared trash icon (identical to Settings clock deletion) opens deletion confirmation directly; editing is available only through the card, without a duplicate menu action. Cancel does not change stored data. Re-enabling a disabled Once recalculates its next date in its saved zone. Completed/missed statuses are deferred until delivery exists. Save failures must not remove or overwrite in-memory alarms.

- **ALM-05:** Creating a Once alarm without a date picker selects the next strictly future occurrence of the entered H:M in the main clock's IANA zone. A passed or equal time rolls to tomorrow, including month/year boundaries. DST gaps resolve to the first valid minute after the gap; overlaps use the earlier instant only. Store the requested local date/time and zone. This slice computes the occurrence but does not deliver notifications or sound.

- **ALM-04:** Override ALM-03's Once label accent with neutral gray; only Weekly is accented. Dates use 13px text. Saved alarm times use 14px text centered vertically beside the toggle. Selected weekdays are bold in the editor and saved weekly rows. Alarm action captions and H:M digits receive a 2px optical downward correction without moving their hit targets.

- **ALM-03:** Saved alarm cards align with the section heading's left edge. Once/Weekly labels use the Planner accent; one-time dates remain below the title and both alarm types show the time right-aligned before the toggle. Planner uses a 40% brighter variant of the shared clock-derived accent, both initially and during live updates. The alarm editor weekday selectors use 14px text without button backgrounds, retaining 42px by 30px hit targets. Selected weekdays are blue and weekends red; unselected days and all days in Once mode are gray. Once disables day selection without clearing it.

- **UI-08:** Planner and Calendar accents update immediately when the main clock color or main clock selection is applied in Settings, including live color preview. Hidden windows retain the updated accent for their next opening; no window recreation or restart is required.

- **UI-07:** Planner can be dragged from non-interactive background areas, including the full header and empty sidebar. Interactive controls (including close, navigation, text editors, and scroll areas) retain their pointer behavior; the resize edge remains available. Selecting Planner from the clock context menu shows, restores, raises, and focuses the existing window without hiding/recreating it or resetting its selected section and drafts.

- **UI-01 (Legacy):** Edit, Settings, About, color picker, rulers, and slider use frameless custom styling and do not create separate taskbar entries; the main clock is the only taskbar entry. Settings and About remain above an always-on-top clock.
- **UI-02 (Legacy):** Position Edit and Settings near the clock strip, preferring below it, falling back above, and constraining them to the working area.
- **UI-03 (Legacy):** The color picker offers the existing curated palette, closes after selection, and dismisses when it loses focus.
- **UI-04:** Dialogs must remain reachable by keyboard; Escape closes Settings, About, and the context menu, Enter accepts Settings and About, and focus indication must be visible.
- **UI-05:** Context-menu and mouse shortcuts must have discoverable menu equivalents.
- **UI-06:** Settings, About, and the custom context menu must take foreground focus above the ruler and slider windows whenever they are shown.

## Calendar Panel

- **CAL-01:** Offer an optional frameless calendar window by clicking the clock strip. It runs in the same process, creates no separate taskbar/dock entry, and toggles independently from Settings and About.
- **CAL-02:** Attach the calendar to the clock strip, centered horizontally. Prefer placement below the strip, fall back above it when required, and constrain the complete calendar to the active monitor working area.
- **CAL-03:** Provide Week, Month, and Year views with Month as the initial view. Previous, Today, and Next navigation reuse the same selected-date state across views.
- **CAL-04:** The calendar provides light and dark themes derived from the main clock accent. Month view distinguishes weekends, supports Monday or Sunday as the first day of the week, shows adjacent-month dates quietly, and highlights today and the selected date.
- **CAL-05:** Week view places Monday through Sunday horizontally and time vertically in a fixed 12-hour viewport. It opens with the current time centered, preserves solid day separators, uses subtle hour guides, and marks the current time with a 12-pixel label when visible. Scrolling advances the focused date day by day; after Sunday it changes to the following Monday and week. The focused date has a circular outline without a fill, while the selected date remains filled. Pointer and trackpad scrolling remain available without visible scrollbars. When the current day moves outside the displayed week, the view returns to that day and recenters its current time.
- **CAL-09:** Calendar theme and first-day-of-week choices are configured in Settings; the calendar panel also provides a direct light/dark theme toggle. The panel contains navigation, view, and theme controls without duplicating the other calendar settings.
- **CAL-10:** A single click on the clock strip toggles the calendar without a double-click delay. The calendar hides when it loses focus. Dragging the clock strip must not show or hide the calendar, including on platforms where the native drag loop suppresses intermediate pointer-move events.
- **CAL-06:** Year view shows twelve readable 7-by-6 mini-months, highlights the current month and date, shows adjacent-month dates quietly, and opens a month when selected.
- **CAL-07:** Calendar date calculations are deterministic Rust logic covered by focused tests. While visible, the calendar refreshes its current time and date marker on the same periodic clock refresh as the clock strip, and does not reset a manually browsed week.
- **CAL-08:** At four or more clocks the calendar may visually join the strip. With one to three clocks it retains a usable minimum width, centers below the strip, and keeps rounded upper corners. Calendar height must not grow merely because clock UI scale or clock count increases.

## Planner Timer Duration Editor

- **ALM-02:** The alarm creation form has a 70-pixel top row: name at the top left, weekdays below it, and the H:M editor on the right spanning both left rows. The fixed-width (74-pixel) Once/Weekly button is right-aligned below the name; resizing increases the gap after Sun, not the button width. Add alarm occupies a separate full-width row below. In Once mode, weekday controls remain visible but disabled, preserving the layout and previous selections.

- **ALM-01:** Alarm creation uses the same single-frame segmented time editor as timers, with H:M only and initial time 07:00. The active segment is highlighted; shared arrow symbols and keyboard Up/Down adjust the current text within 0–23 hours and 0–59 minutes without wrapping. Tab and direct entry remain available. Both one-time and weekly alarms store normalized HH:MM values; invalid or incomplete times are rejected and midnight is valid.

- **TMR-01:** Create timer durations using a single framed D:H:M:S control. Defaults are 0:0:5:0. Each segment supports direct text entry, with the active segment highlighted; one shared pair of unfilled arrow symbols adjusts the active segment. Keyboard Up/Down provides the same adjustment and Tab moves between segments.
- **TMR-02:** Days range from 0 to 999, hours from 0 to 23, and minutes/seconds from 0 to 59. Adjustment stops at the limits without wrapping or carrying into other segments. Invalid or all-zero durations do not create a timer. Adjustments use the current text, never stale stored values.
- **TMR-03:** Remember the last valid edited duration across section changes, timer creation, and application restart; incomplete or invalid text must not overwrite the last valid stored value.

## Cross-Platform Quality Requirements

- **NFR-01:** Support current Windows, macOS, and mainstream Linux desktop releases.
- **NFR-02:** Preserve proportions, baselines, one-pixel edges, gradients, and seamless tile joins at 100%, 125%, 150%, and 200% scale.
- **NFR-03:** Use golden-image tests for the three reference modes and unit tests for time conversion, DST boundaries, ordering, removal rules, and persistence migration.
- **NFR-04:** Keep idle CPU use near zero apart from the once-per-second clock update; extended interactive mode may update more frequently only while visible.
- **NFR-05:** Work without network access and collect no telemetry by default.
- **NFR-06:** Use platform-appropriate context menus, tray/status items, startup behavior, packaging, and signing without leaking platform APIs into domain logic.
- **NFR-07:** Let users opt into launching OziClock automatically at system login through a persisted Settings option.

## Legacy Gaps to Avoid

The WPF source has no tray icon despite supporting `ShowInTaskbar = false`. Its topmost code contains an unresolved TODO, and disabling the setting does not explicitly demote an already-topmost window. Settings objects do not notify all bound views, so taskbar visibility and seconds may not update reliably at runtime. The rewrite must implement the requirements above rather than reproduce these limitations.
