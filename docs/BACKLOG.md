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
