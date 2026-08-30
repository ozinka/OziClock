# Future Feature Envelope

This document reserves architectural space without committing the first release to every feature. Each feature requires its own detailed requirements before implementation.

Actionable work, priority, and status are tracked in [BACKLOG.md](BACKLOG.md). When the two documents differ, the backlog determines whether an idea is currently queued, while this document continues to describe its broader product envelope.

## Candidate Capabilities

### Platform Polish

- **Live edge snapping:** While dragging, visibly snap the clock strip to monitor work-area edges. The implementation must use the platform drag lifecycle rather than cursor polling, preserve compact-mode geometry, and remain smooth on high-DPI displays.
- **DPI-aware taskbar icon:** Ensure Windows chooses the appropriate frame from the multi-resolution legacy ICO (16/32/48/64) without flattening it to one bitmap. Verify visual parity with the WPF taskbar icon at common display scales before enabling it.

### Calendar Panel

- Add an optional calendar panel for quickly checking dates without turning the clock strip into a permanent dashboard.
- Explore week, month, and year views. The month view is the likely default; week and year views should reuse the same navigation and selection language.
- Visually distinguish weekends while keeping ordinary weekdays quiet. The first day of the week must be configurable as Monday or Sunday, with Monday as the default.
- Reuse the visual character of the clock: compact typography, a pleasant dark-to-accent surface, and subtle edge shading or fading. Derive the accent from the main clock by default while preserving readable contrast.
- Keep date selection and calendar navigation independent from reminder behavior. A selected date may become an entry point for reminders later, but the first calendar iteration does not create or schedule them.
- Prototype the interaction and proportions in HTML before implementing the Slint component. Compare attached-panel and independently positioned variants, all three time ranges, weekend emphasis, navigation density, and behavior at different clock scales.
- Avoid background polling. The panel should derive its current-date state from the shared application clock and request refresh only at the next relevant date boundary or when opened.

The approved prototype resolves the initial layout decisions: the calendar is a separately hosted frameless window attached to the whole strip, Month is the initial view, Week and Year are persistent modes, and adjacent-month dates remain visible with quiet styling. Locale-specific weekend definitions remain a possible later enhancement.

### Alarms

- One-time or recurring local-time alarms.
- Explicit time zone and daylight-saving behavior.
- Enable, disable, snooze, dismiss, and missed-alarm handling.
- Native notification with sound policy where supported.

### Reminders

- Message attached to an absolute instant or relative delay.
- Optional recurrence and association with a configured world clock.
- Notification history sufficient to avoid duplicate delivery after restart.

### Countdown Timers

- Multiple named timers with start, pause, resume, reset, and optional repeat.
- Accurate completion while the window is hidden or the computer sleeps.
- Compact reusable controls that can appear in the clock strip or a feature panel.

### Stopwatch

- Start, pause, resume, reset, and lap capture.
- Monotonic elapsed-time measurement.
- Optional persistence of paused sessions; running-session recovery requires an explicit product decision.

## Product Boundaries

World clock remains the primary identity of OziClock. Extra tools must be optional and must not increase idle work when disabled. They should use the same tray, notification, persistence, scheduler, typography, and control primitives. The main clock strip must not become a generic dashboard by default.

## Required Decisions Before Implementation

For each capability, define interaction design, compact/expanded placement, persistence and restart behavior, operating-system notification behavior, sound policy, missed-event semantics, and acceptance tests. Record irreversible or cross-cutting choices as ADRs.
