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

- **ALM-19:** Every Delivered alarm receipt starts as unacknowledged. The local attention queue restores all unacknowledged deliveries in occurrence order after restart. Dismiss persists acknowledgement before advancing the queue; Snooze atomically persists both acknowledgement of the shown occurrence and its replacement deadline. A save failure keeps the card visible. Missed and acknowledged receipts never enter the attention queue.

- **ALM-18:** Alarm occurrence history is retained for 30 days based on each receipt's recorded UTC time. The exact 30-day boundary remains available; older valid receipts are pruned before scheduler persistence. A malformed timestamp is retained rather than silently deleting data.

- **ALM-17:** Each saved alarm card shows the latest persisted occurrence result: Delivered in the Planner accent or Missed in red. The latest result is selected by recorded UTC time and remains visible across restart. Alarms without receipts show no status. This is history feedback and does not replace On/Off.

- **ALM-16:** The local alarm card offers Snooze 5 min and Dismiss. Snooze stores one replacement UTC deadline per alarm without changing its Once/Weekly rule; repeated snooze replaces that deadline. Snoozes survive restart, use the same five-minute delivery grace and receipt de-duplication, and may apply after Once has become Off. Snooze is committed before the attention card advances. Deleting an alarm deletes its pending snooze.

- **ALM-15:** While the local alarm-attention card exists, only its accent border pulses smoothly between one and three pixels every 450ms. The surface, text and controls remain stable and fully readable; dismissing the final queued alarm hides the card.

- **ALM-14:** On each transition into Alarms, when no existing alarm is being edited, initialize the new-alarm H:M from the current time in the main clock zone: round strictly upward to the next five-minute boundary when seconds are present (or retain an exact boundary), then add five minutes, wrapping midnight. Examples: 10:02:00 → 10:10, 10:05:00 → 10:10, 10:05:01 → 10:15, 23:58 → 00:05. Entering Alarms during editing preserves the alarm's stored time.

- **ALM-13:** A single Slint timer evaluates alarm intervals once per second. It atomically saves reconciliation changes before exposing deliverable occurrences. Delivered alarms enter a FIFO local-attention queue shown in a compact always-on-top card with a prominent alarm icon adjacent to the clock strip; Dismiss advances to the next item or closes the card. Planner never opens automatically and the adapter does not explicitly request focus. Once becomes Off when reconciled. Persistence failure prevents both in-memory mutation and attention display. Native notification, sound and Snooze remain separate slices.

- **ALM-12:** Planner persists alarm occurrence receipts containing alarm ID, occurrence UTC, recorded UTC and Delivered/Missed status, plus the last scheduler check instant. Reconciliation records each occurrence at most once, classifies occurrences within a five-minute grace window as deliverable and older ones as missed, and disables a reconciled Once alarm. Deleting an alarm also removes its receipts. Saving the receipt must precede notification delivery; this slice does not yet invoke a platform notification adapter.

- **ALM-11:** Scheduler evaluation accepts an explicit previous and current UTC instant and returns enabled alarm occurrences in the half-open interval `(previous, current]`, sorted by occurrence time. This prevents duplicate delivery at a boundary and permits deterministic sleep/wake reconciliation. Evaluation is side-effect free; durable receipts and delivery own mutations.

- **ALM-10:** The application layer calculates the next strictly future occurrence for enabled Once and Weekly alarms in their saved IANA zone. Weekly searches today through the following seven days and skips today's occurrence after its time passes. Disabled and expired Once alarms have no next occurrence. DST uses ALM-05 policies. This calculation is the scheduler input; delivery is a separate slice.

- **ALM-09:** Clicking an alarm card selects it without entering edit mode and reveals Edit in the lower action slot. Selection uses the same darker card background as timers. Clicking Edit loads the alarm into the form, adds a thin accent border, and changes that action to Cancel. Cancel exits editing but keeps the alarm selected so Edit remains available. Clicking the selected card again deselects it and exits editing. On/Off and Delete do not select the card.

- **ALM-08:** Override ALM-07's fixed left column: its minimum is 354px and it alone absorbs additional width. Weekday hit targets are 38px wide with 2px spacing, followed immediately by the fixed Once/Weekly control. H:M and the 125px action column remain fixed-width and move right as the window expands. The edited-card accent border is one pixel.

- **ALM-07:** The alarm editor uses a fixed 403px left column so Once/Weekly follows Sun directly. The H:M editor remains in the middle. Add alarm/Save changes occupies the top of the flexible right column, with Cancel directly below only while editing. The edited saved-alarm card is identified by a two-pixel accent border.

- **ALM-06:** Alarm cards open editing in the existing footer with Save changes and Cancel editing. Saving preserves identity and saved zone and recalculates Once's next date. An enabled alarm remains enabled. A disabled Once whose current occurrence has a matching Delivered/Missed receipt is re-enabled when edited, creating a new future occurrence; a manually disabled alarm without that receipt and every disabled Weekly alarm remain disabled. A trailing shared trash icon (identical to Settings clock deletion) opens deletion confirmation directly; editing is available only through the card, without a duplicate menu action. Cancel does not change stored data. Re-enabling a disabled Once recalculates its next date in its saved zone. Save failures must not remove or overwrite in-memory alarms.

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
- **TMR-04:** A running timer persists its remaining duration and UTC start anchor. After restart, the application reconstructs the remaining seconds from the current UTC instant without consuming time when the clock moved backward or the anchor is invalid. Expired running timers transition to Finished exactly once and persist that transition; UI refresh code does not own countdown arithmetic.
- **TMR-05:** Every Finished timer enters a persistent local attention queue restored after restart. Its compact always-on-top card shows a prominent timer icon, the timer title, Dismiss, and Restart. Dismiss persists a transition from Finished to inactive Idle before advancing the queue; Restart restores the original duration, starts from the current UTC instant, persists, then advances. Starting the same finished timer from Planner also removes its stale attention card. Save failure keeps the current card visible.
- **TMR-06:** A timer has a required custom title and may be selected independently from editing. Saved cards and completion attention show the original duration after the name, separated by an em dash, while editing loads only the unchanged custom name. Edit loads its title, original duration, and repeat setting into the shared editor; saving resets the countdown to the edited full duration. Cancel leaves the stored timer unchanged.
- **TMR-07:** Every saved timer exposes Reset and Delete. Reset returns it to inactive Idle at its full duration and clears pending attention. Delete requires confirmation, removes the timer durably, and closes any attention card for it.
- **TMR-08:** A timer may repeat for an arbitrary positive number of additional runs or indefinitely. The repeat count excludes the initial run, is persisted with the remaining count, and Reset or Restart restores the configured count. On expiry it raises one persistent attention card and, while repetitions remain, immediately starts its next full interval. Dismiss acknowledges the card without stopping an already-running next interval; Restart acknowledges it and starts a fresh full sequence. An unacknowledged card is not duplicated by later repeat cycles.
- **TMR-10:** Timer attention uses the same animated accent-border pulse as alarm and reminder attention. Its upper-right corner shows the first unacknowledged completion instant in the main clock's local time; later coalesced repeat cycles do not replace that timestamp.
- **TMR-09:** The saved-timer list uses all available panel height and scrolls only when its cards exceed that height. Row actions remain inside the content width and do not overlap the scrollbar.

## Planner Tasks

- **TSK-01:** Create an open task from Plan or Tasks using a required trimmed title. Empty titles show inline validation in Tasks and do not change persisted data.
- **TSK-02:** The Tasks panel uses all available height and scrolls only when the active status list exceeds it. Empty Open and Completed lists have distinct messages.
- **TSK-03:** Clicking an open task selects or deselects it using the same darker-card treatment as alarms and timers, without immediately entering edit mode.
- **TSK-04:** Edit loads the selected open task title into the shared input. Save changes persists the trimmed title; Cancel and leaving Tasks discard the draft without changing the task.
- **TSK-05:** Completing an open task removes it from Open and makes it available under Completed without deleting it.
- **TSK-06:** Open and Completed are explicit status views with live counts. Switching views clears stale selection and editing state.
- **TSK-07:** Reopen moves a completed task back to Open and preserves its title and metadata.
- **TSK-08:** Archive removes a completed task from ordinary task views while retaining it in local Planner storage.
- **TSK-09:** Open and completed tasks expose Delete through the shared trash icon and require confirmation before permanent removal.
- **TSK-10:** Create, rename, complete, reopen, archive, and delete save the new Planner state before replacing the in-memory state and visible models. A save failure preserves the prior task collection.

## Planner Stopwatch

- **SW-01:** Lap capture persists the cumulative elapsed time to millisecond precision and displays hundredths. Existing whole-second lap data remains readable and migrates when the next lap is recorded.
- **SW-02:** Each lap row shows both its cumulative Total and its Split since the preceding lap; the first split starts at zero.
- **SW-03:** Lap rows retain stable sequence numbers and show the newest lap first so the latest result stays visible without scrolling.
- **SW-04:** With at least two unequal splits, every fastest split is emphasized with the Planner accent and a subtle positive background.
- **SW-05:** With at least two unequal splits, every slowest split is emphasized in red with a subtle warning background. Equal splits remain neutral rather than being both fastest and slowest.
- **SW-06:** Undo removes only the latest recorded lap without changing elapsed time or the running/paused state, and is disabled when no laps exist.
- **SW-07:** Clear removes all lap history without resetting elapsed time or changing the running/paused state, and is disabled when no laps exist.
- **SW-08:** Lap is enabled only while the stopwatch is running. Reset is enabled only when elapsed time or lap history exists; disabled actions use visibly muted styling.
- **SW-09:** Reset requires confirmation and clears elapsed time and every lap only after confirmation. Cancel and backdrop dismissal preserve the session.
- **SW-10:** Stopwatch and lap formatting supports sessions beyond 24 hours using a day prefix while preserving H:M:S and hundredths for laps.

## Planner Reminders

- **REM-01:** Create a reminder for an absolute local date and time using a YYYY-MM-DD field and a segmented H:M editor. Entering the section proposes a time ten minutes in the future in the main clock's time zone.
- **REM-02:** A reminder requires a non-empty trimmed title, a valid local date and time, and a future occurrence. Invalid input shows an inline error without changing persisted data.
- **REM-03:** Saved reminders use all available panel height, scroll as needed, and show their local date, time, and source time zone. Disabled reminders are labelled Paused and delivered reminders are labelled Delivered.
- **REM-04:** Clicking a reminder selects it without editing. Edit loads its title and original local date and time; Save changes schedules the chosen future occurrence, while Cancel preserves the stored reminder.
- **REM-05:** A pending reminder may be switched On or Off without changing its due instant. A delivered reminder cannot be re-enabled without editing and scheduling a new future occurrence.
- **REM-06:** Delete uses the shared trash icon and confirmation overlay, removes the reminder durably, and closes its attention card if currently queued.
- **REM-07:** A single one-second scheduler evaluates absolute UTC deadlines. A due reminder is atomically persisted as Off with attention pending before its card is displayed, and cannot be delivered twice.
- **REM-08:** Due reminders enter a chronological FIFO attention queue. Its compact always-on-top card uses a distinct large reminder icon and Dismiss advances to the next reminder.
- **REM-09:** Pending reminder attention is durable and restored in chronological order after application restart. Save failure leaves the prior in-memory state and attention display unchanged.
- **REM-10:** Editing a delivered reminder clears its delivered state, schedules the chosen future occurrence, enables it, and removes any stale attention item.
- **REM-11:** The date remains directly editable and has an adjacent calendar icon. Activating it opens a Monday-first six-week month picker focused on the entered date (or today when invalid); past dates are disabled, today is outlined, the selected date uses the accent, adjacent-month days remain visible, and choosing a date updates the field and closes the picker.
- **REM-12:** A reminder recurrence menu offers Once, Daily, Weekly, Monthly, and Yearly. Weekly exposes a multi-select weekday row and requires at least one selected day; the entered date is the earliest eligible occurrence.
- **REM-13:** After a recurring reminder is delivered, it remains enabled and its next UTC occurrence is persisted before attention is shown. Occurrences that pass while its attention card remains pending are coalesced rather than queued repeatedly.
- **REM-14:** Monthly recurrence preserves the selected day number and uses the month's final day when that number is absent. Yearly February 29 occurrences use February 28 in non-leap years and return to February 29 in leap years.
- **REM-15:** Every recurrence resolves wall time in its saved IANA time zone. DST gaps advance to the first valid minute and overlaps choose the earlier occurrence, matching one-time reminder behavior.

## Planner Schedule View

- **PLN-01:** Week view derives its Monday-to-Sunday dates from the main clock's local date. Previous and next move exactly seven days; activating the range label returns to the current week.
- **PLN-02:** Enabled reminders whose next occurrence falls between 09:00 and 18:00 in the main clock's time zone appear as accent markers at the corresponding day and time.
- **PLN-03:** Clicking an empty hourly cell opens the existing Reminders editor with that local date and hour prefilled as a one-time reminder. The Plan view does not duplicate reminder validation or persistence.
- **PLN-04:** Alarm, timer, reminder, and task lists keep creation forms out of the page layout. Add opens a dimmed in-window modal, Edit appears only for the selected active item, and double-click opens the same modal directly. Saving closes the modal; Cancel, the close icon, or backdrop dismissal preserves stored data and returns the selected item to its non-editing state.
- **PLN-05:** Plan reserves a compact Events area for a future personal calendar-event workflow. The placeholder does not accept input or imply participant, invitation, or synchronization support. The sidebar groups Plan, Alarms, Reminders, and Tasks together, then separates Timers and Stopwatch with a subtle divider.

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
