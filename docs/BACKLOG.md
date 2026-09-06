# OziClock Backlog

This is the canonical work queue for planned OziClock changes. Product behavior and acceptance criteria remain authoritative in `REQUIREMENTS.md` and feature design documents. `FUTURE_FEATURES.md` is an idea envelope, not a commitment to implement every item.

## How We Use This Backlog

- Keep an item's ID stable across discussions, commits, and pull requests.
- Move an item to `Ready` only after its intended behavior and acceptance criteria are clear.
- Add or update requirement IDs before implementation when an item changes observable behavior.
- Keep only one status per item: `Candidate`, `Discovery`, `Ready`, `In Progress`, `Blocked`, or `Done`.
- Record dependencies and blocking decisions in the item rather than in source-code TODO comments.
- Move completed items to the archive at the bottom of this file.

Priorities describe sequencing, not severity:

- `P1` — product quality or correctness work to address next.
- `P2` — focused product improvement after P1 work.
- `P3` — larger optional capability requiring its own product slice.

## Queue

| ID | Priority | Status | Area | Item | Next step |
| --- | --- | --- | --- | --- | --- |
| BL-045 | P3 | Candidate | Project funding | Add a store-publication donation option to README | Choose a trusted donation platform and add a concise README section explaining that contributions can help cover publishing OziClock in application stores. State intended use transparently without promising a release date or store acceptance. |
| BL-051 | P2 | In Progress | Tasks | Complete the local task lifecycle | TSK-01–10 implement validated creation, status views and counts, selection/edit/cancel, completion, reopen, archive, confirmed deletion, responsive scrolling and save-before-display persistence. Visual verification pending. |
| BL-052 | P2 | In Progress | Stopwatch | Add precise lap analysis and safe controls | SW-01–10 implement hundredth-precision totals and splits, newest-first numbering, fastest/slowest emphasis, Undo, Clear, state-aware actions, confirmed Reset, legacy-lap migration and multi-day formatting. Visual verification pending. |
| BL-053 | P1 | In Progress | Reminders | Implement durable dated reminders | REM-01–11 implement absolute local date/time entry with a mini calendar, validation, source-zone display, edit/rearm, On/Off, confirmed deletion, atomic scheduling, chronological local attention and restart recovery. Visual verification pending. |
| BL-054 | P1 | In Progress | Reminders | Add recurring reminder schedules | REM-12–15 implement daily, selected-weekday weekly, monthly and yearly recurrence with durable next occurrences, end-of-month/leap-day rules, DST handling and coalescing while attention is pending. Visual verification pending. |
| BL-055 | P1 | In Progress | Plan | Connect reminders to the weekly schedule | PLN-01–03 implement live week dates, navigation, enabled reminder markers and prefilled creation through the shared Reminders editor. Month/year integration and overlap layout remain later slices. |
| BL-056 | P1 | In Progress | Planner editors | Move creation and editing into shared modals | PLN-04 replaces persistent bottom forms for alarms, timers, reminders and tasks with Add/conditional Edit actions, double-click editing and in-window modal editors. Visual verification pending. |
| BL-057 | P1 | Candidate | Tasks | Add scheduling and task alerts | Extend the task editor with optional due date/time, scheduled start/end and alert offsets such as at due time, 15 minutes before and Custom. Deliver alerts through the shared scheduler while preserving task identity and completion state. |
| BL-049 | P2 | In Progress | Timers | Add repeat countdowns | TMR-08 implemented with immediate next-cycle restart and one durable attention item; verify Dismiss and Restart while the next interval is running. |
| BL-050 | P2 | In Progress | Timers | Scale the saved-timer list | TMR-09 implemented with available-height scrolling and reserved scrollbar space; visual verification pending. |
| BL-048 | P2 | In Progress | Timers | Edit named countdown timers | TMR-06 implemented with independent selection, edit/cancel, title validation and duration reset; visual verification pending. |
| BL-047 | P2 | In Progress | Timers | Reset and safely delete timers | TMR-07 implemented with per-row Reset and confirmed trash deletion; visual and persistence verification pending. |
| BL-046 | P1 | In Progress | Timer attention | Persist Finished cards with Restart and Dismiss | TMR-05 implemented with a dedicated local timer-attention queue restored from durable state; verify restart, multiple completions and both actions. |
| BL-044 | P1 | In Progress | Timers | Recover running countdowns after restart | TMR-04 implemented with deterministic application-layer remaining-time calculation and expiry reconciliation; verify restart while running and restart after the deadline. |
| BL-043 | P1 | In Progress | Alarm attention | Restore unacknowledged alarm cards after restart | ALM-19 persists acknowledgement for Dismiss/Snooze and rebuilds the FIFO attention queue from Delivered receipts; verify restart before dismissal and queue advancement. |
| BL-042 | P2 | Candidate | Alarm delivery | Add native operating-system notifications | Define and test a platform adapter with permission handling, non-crashing fallback, title/time content and notification actions. On macOS use `UNUserNotificationCenter`; the deprecated AppleScript-dependent backend crashed inside the Slint event loop and must not be used. Keep the local attention card as the active delivery surface until this slice is promoted. Sound remains separate. |
| BL-041 | P2 | In Progress | Alarm history | Retain occurrence receipts for 30 days | ALM-18 implemented in the storage adapter and scheduler persistence path; verify recent status persistence after restart. |
| BL-040 | P1 | In Progress | Alarm history | Expose the latest delivery result in saved alarms | ALM-17 implemented; verify Delivered/Missed placement and persistence for Once and Weekly. |
| BL-039 | P2 | Candidate | Alarm delivery | Add configurable Snooze durations | Define a global default, optional per-alarm override, quick choices such as 5/10/15 minutes and Custom on the attention card, validation limits, and whether the last quick choice is remembered. Keep the current five-minute Snooze until this slice is promoted. |
| BL-038 | P1 | In Progress | Alarm delivery | Add persisted five-minute Snooze | ALM-16 implemented with replacement, restart, receipt and deletion semantics; verify Snooze card recurrence after five minutes. |
| BL-037 | P2 | In Progress | Alarm attention | Pulse the local alert border | ALM-15 implemented; visually verify prominence and comfort during a triggered alarm. |
| BL-036 | P2 | In Progress | Alarm editor | Initialize new alarms from a useful near-future time | ALM-14 implemented with main-zone and midnight tests; verify section re-entry and editing preservation. |
| BL-035 | P1 | In Progress | Alarm delivery | Run the scheduler and show local alarm attention | ALM-13 implemented with save-before-display and FIFO Dismiss; verify a near-future Once/Weekly alarm and non-activation. Next add native notification, sound and Snooze. |
| BL-034 | P1 | In Progress | Alarm scheduling | Persist delivery and missed occurrence receipts | ALM-12 model and deterministic reconciliation implemented; next wire atomic persistence before local/native delivery and expose status in Planner. |
| BL-033 | P1 | In Progress | Alarm scheduling | Reconcile due alarms over a clock interval | ALM-11 deterministic due-interval evaluation implemented; next add durable occurrence receipts and missed/delivered transitions before notification adapters. |
| BL-032 | P1 | In Progress | Alarm scheduling | Calculate the next Once or Weekly occurrence | ALM-10 implemented with deterministic zone, disabled, expiry, weekday and DST tests; connect the result to the single application scheduler. |
| BL-031 | P2 | In Progress | Alarms | Separate alarm selection from editing | ALM-09 implemented; verify select/deselect and Edit/Cancel transitions without affecting On/Off or Delete. |
| BL-030 | P2 | In Progress | Alarms | Make the compact editor responsive without stretching actions | ALM-08 implemented; verify minimum and expanded widths plus one-pixel edited-card highlight. |
| BL-029 | P2 | In Progress | Alarms | Compact and clarify the alarm editor layout | ALM-07 implemented; verify resizing, right-side actions, and edited-card highlight. |
| BL-028 | P1 | In Progress | Alarms | Edit, confirm deletion and rearm Once | ALM-06 re-enables an edited handled Once by matching its occurrence receipt, while preserving manual Off for unhandled Once and Weekly alarms; verify edit-after-delivery recurrence, trash, cancel and persistence. |
| BL-027 | P1 | In Progress | Alarm scheduling | Select the next future Once occurrence | ALM-05: application-layer wall-time resolution and creation integration; verify today/tomorrow behavior. Delivery, receipts, snooze and sound remain in BL-007. |
| BL-026 | P2 | In Progress | Alarms | Refine typography and optical alignment | ALM-04 implemented; verify larger dates/times, bold selected days, neutral Once, and action/input alignment. |
| BL-025 | P2 | In Progress | Alarms | Align saved cards and clarify schedule/time styling | ALM-03 implemented, including text-only 14px weekday selectors and a 40% brighter accent; visual verification pending. |
| BL-024 | P1 | In Progress | Planner appearance | Update the accent live after clock settings changes | UI-08 implemented in Apply and Save; verify live color preview and switching the main clock in the debug build. |
| BL-023 | P2 | In Progress | Planner windowing | Drag from the background and reactivate Planner from the menu | UI-07 implemented; all 34 tests and build pass. Latest debug launched; user verification of dragging, control hit targets, and foreground activation remains. |
| BL-022 | P2 | In Progress | Alarms | Reuse the segmented H:M time editor for alarms | ALM-01 and ALM-02 implemented; 34 tests and build pass. Compact two-row layout awaits user visual confirmation in the latest debug instance. |
| BL-021 | P2 | Done | Timers | Implement the approved single-field D:H:M:S editor | TMR-01–03 implemented. Focused parsing/adjustment and persistence tests pass; macOS visual smoke check verified alignment, shared arrows, segment selection, and keyboard adjustment. |
| BL-003 | P2 | Discovery | Windowing | Add live edge snapping while dragging | Define per-platform drag-lifecycle behavior and acceptance tests for compact mode and high DPI. |
| BL-004 | P2 | Ready | Windows | Preserve multi-resolution frames in the taskbar icon | Verify 16/32/48/64-pixel ICO selection against WPF at common display scales. |
| BL-013 | P1 | Discovery | macOS windowing | Diagnose the clock strip moving down when resized or restored | Compact mode, rulers, and relaunch/login restore can shift the window downward by about 30 physical pixels at 85% clock scale. Establish the native frame/content-coordinate model and an acceptance test before changing production window behavior. |
| BL-014 | P2 | Done | Startup | Launch OziClock automatically with the operating system | Implemented persisted Settings toggle and native startup registration for macOS, Windows, and Linux. |
| BL-007 | P3 | Candidate | Alarms | Add one-time and recurring alarms | Define recurrence, DST, snooze, missed-alarm, sound, notification, and restart semantics. |
| BL-008 | P3 | Candidate | Reminders | Add absolute and relative reminders | Define recurrence, clock association, notification history, and duplicate-delivery prevention. |
| BL-009 | P3 | Candidate | Timers | Add multiple named countdown timers | Define sleep/restart recovery, repeat behavior, persistence, and compact controls. |
| BL-010 | P3 | Candidate | Stopwatch | Add stopwatch and lap capture | Define paused-session persistence and running-session recovery before writing requirements. |
| BL-016 | P3 | Discovery | Planner | Design local-first alarms, timers, stopwatch, reminders, and tasks | Agreed prototype and design are recorded; add requirement IDs and acceptance tests before promoting the independent vertical slices. |
| BL-017 | P3 | Candidate | Planner data | Export and import portable Planner archives | Define archive schema, validation/preview, five-day rolling backups, conflict choices, backup recovery, and `.ics` compatibility scope. |
| BL-018 | P3 | Candidate | Planner sync | Add multi-device Planner synchronization through a Google Drive/OneDrive folder | Define immutable operation records, folder scanning, per-entity merge/tombstone policy, encrypted-record handling, offline recovery, and conflict-review UX; direct OAuth sync is later. |
| BL-019 | P3 | Candidate | Planner security | Add optional password-encrypted Planner storage | Define encrypted envelope versioning, audited crypto dependencies, unlock/lock policy, key rotation, encrypted backups/archives, and cloud merge behavior. |
| BL-020 | P2 | Discovery | Planner windowing | Revisit adaptive Planner window sizes | Define a flicker-free, cross-platform transition between compact tools such as Stopwatch and full planning views; preserve user resizing and verify no temporary window disappearance on macOS. |

## Definition of Ready

An item is `Ready` when:

- its user-visible outcome and exclusions are explicit;
- relevant requirement IDs exist;
- cross-platform behavior and persistence are decided where applicable;
- acceptance tests are identified;
- architectural ownership and dependencies are known;
- no unresolved product choice would materially change the implementation.

## Completed Archive

| ID | Completed | Item | Evidence |
| --- | --- | --- | --- |
| BL-015 | 2026-09 | Fade the clock and system shadow together on macOS | WIN-06A: native window alpha implemented; user confirmed improved fading with the system shadow enabled. |
| BL-011 | 2026-08 | Native macOS application icon | `OziClock.icns` is packaged and declared through `CFBundleIconFile`. |
| BL-012 | 2026-08 | Initialize the color picker from the current color | The desktop adapter converts the current color to HSV and positions all picker controls when opened. |
| BL-005 | 2026-08 | Searchable IANA time-zone selection | CLK-08A is implemented with case-insensitive ID/display filtering, preserved ordering, empty-state feedback, keyboard-safe focus handling, and focused unit tests. |
| BL-001 | 2026-08 | Remove clock-strip edge artifacts at scaled sizes | Confirmed complete after implementation and visual review. |
| BL-002 | 2026-08 | Make ruler ticks pixel-perfect at common display scales | Confirmed complete after implementation and visual review. |
| BL-006 | 2026-08 | Add an optional calendar panel | CAL-01 through CAL-09 are implemented with Week, Month, and Year views, persisted settings, live settings updates, and focused calendar tests. |
