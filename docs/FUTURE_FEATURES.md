# Future Feature Envelope

This document reserves architectural space without committing the first release to every feature. Each feature requires its own detailed requirements before implementation.

## Candidate Capabilities

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

