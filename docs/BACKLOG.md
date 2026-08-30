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
| BL-003 | P2 | Discovery | Windowing | Add live edge snapping while dragging | Define per-platform drag-lifecycle behavior and acceptance tests for compact mode and high DPI. |
| BL-004 | P2 | Ready | Windows | Preserve multi-resolution frames in the taskbar icon | Verify 16/32/48/64-pixel ICO selection against WPF at common display scales. |
| BL-007 | P3 | Candidate | Alarms | Add one-time and recurring alarms | Define recurrence, DST, snooze, missed-alarm, sound, notification, and restart semantics. |
| BL-008 | P3 | Candidate | Reminders | Add absolute and relative reminders | Define recurrence, clock association, notification history, and duplicate-delivery prevention. |
| BL-009 | P3 | Candidate | Timers | Add multiple named countdown timers | Define sleep/restart recovery, repeat behavior, persistence, and compact controls. |
| BL-010 | P3 | Candidate | Stopwatch | Add stopwatch and lap capture | Define paused-session persistence and running-session recovery before writing requirements. |

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
| BL-011 | 2026-08 | Native macOS application icon | `OziClock.icns` is packaged and declared through `CFBundleIconFile`. |
| BL-012 | 2026-08 | Initialize the color picker from the current color | The desktop adapter converts the current color to HSV and positions all picker controls when opened. |
| BL-005 | 2026-08 | Searchable IANA time-zone selection | CLK-08A is implemented with case-insensitive ID/display filtering, preserved ordering, empty-state feedback, keyboard-safe focus handling, and focused unit tests. |
| BL-001 | 2026-08 | Remove clock-strip edge artifacts at scaled sizes | Confirmed complete after implementation and visual review. |
| BL-002 | 2026-08 | Make ruler ticks pixel-perfect at common display scales | Confirmed complete after implementation and visual review. |
| BL-006 | 2026-08 | Add an optional calendar panel | CAL-01 through CAL-09 are implemented with Week, Month, and Year views, persisted settings, live settings updates, and focused calendar tests. |
