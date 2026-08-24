# Target Architecture

## Architectural Goals

The rewrite must remain lightweight, predictable, and easy to extend through AI-assisted development. New features must compose existing domain services and reusable UI components instead of adding behavior directly to windows. The application remains one native desktop process; modularity does not imply microservices or a runtime plugin system.

Use Rust for domain and application logic and Slint for declarative presentation. Apply ports and adapters only at real boundaries: persistence, operating-system integration, notifications, and time sources. Avoid both god objects and one-trait-per-function abstraction.

## Dependency Direction

```text
Slint UI adapter ───────> Application ───────> Domain
                                ▲
Platform adapter ───────> application ports
Storage adapter ────────> application ports

Desktop composition root wires all adapters together.
```

Dependencies point inward. Domain code must not import Slint, window handles, JSON, tray libraries, or operating-system APIs. Platform and storage adapters implement ports owned by the application layer. UI sends typed commands and renders immutable view state; it does not calculate time zones, write settings, or call platform adapters directly.

## Proposed Cargo Workspace

```text
apps/oziclock-desktop/       Composition root and executable
crates/oziclock-domain/      Clocks, alarms, timers, stopwatch, shared time types
crates/oziclock-app/         Use cases, commands, events, scheduler orchestration
crates/oziclock-ui/          Rust/Slint presentation adapter and view state
crates/oziclock-platform/    Tray, windows, notifications, startup, OS capabilities
crates/oziclock-storage/     Versioned settings and migration
ui/                          Reusable .slint components and design tokens
tests/golden/                Visual reference images
legacy/dotnet-wpf/           Preserved WPF reference implementation
```

Do not create all crates before they carry real behavior. Begin with `domain`, `app`, `ui`, and the desktop composition root; extract platform and storage crates when their adapters are implemented.

## Domain Modules

### World Clock

Owns clock identity, label, color, order, main-zone selection, UTC-to-local conversion, and display preferences. A `ClockTile` UI component consumes a clock view model; it never owns domain state.

### Alarm and Reminder

An alarm represents a target wall-clock time, recurrence rule, time zone, enabled state, and notification policy. A reminder adds user text and may target an absolute instant or a relative duration. Daylight-saving gaps and repeated local times require explicit policies rather than implicit platform behavior.

### Timer

A timer is a state machine: `Idle -> Running -> Paused -> Finished`, with reset and optional repeat. Persist duration and state carefully; reconstruct remaining time from timestamps rather than decrementing a stored counter every second.

### Stopwatch

A stopwatch is a state machine: `Idle -> Running -> Paused`, with reset and laps. Measure elapsed time with a monotonic clock so wall-clock or time-zone changes cannot alter the result.

## Shared Time and Scheduling

Provide injectable `UtcClock` and `MonotonicClock` interfaces. Production adapters use the operating system; tests use deterministic fakes. A single application scheduler owns wake-ups for clocks, alarms, reminders, and timers. Features register deadlines or display refresh needs instead of creating independent polling loops.

The scheduler must sleep until the nearest deadline, wake after system resume, reconcile missed events, and publish typed application events. Normal clocks refresh once per second only when seconds are visible; hidden UI must not redraw unnecessarily.

## State and Communication

Use typed `Command`, `Event`, and `ViewState` values:

```text
User gesture -> Command -> Use case -> Domain change
                                  -> Event
                                  -> ViewState / persistence / notification
```

Commands express intent (`AddClock`, `StartTimer`); events describe facts (`ClockAdded`, `TimerFinished`). Components communicate through the application layer, never by reaching into another component’s fields. Prefer explicit ownership and message passing over global mutable state.

## UI Composition

Maintain reusable components such as `ClockTile`, `ClockStrip`, `RulerColumn`, `RulerLens`, `TimeSlider`, `DurationEditor`, `TransportControls`, and `NotificationBadge`. Centralize dimensions, typography, colors, gradients, animation durations, and DPI rules in design tokens. Feature panels compose these primitives rather than cloning markup.

The main shell owns display modes and placement. Feature modules contribute view state and commands; they do not manipulate the native window directly. Platform capabilities are surfaced to the UI so unsupported behavior can be disabled gracefully.

## Persistence

Persist one versioned application document using stable IDs and IANA time-zone names. Each feature owns its serializable settings section and migration function. Writes must be atomic. Runtime-only values such as window handles, active animation frames, and monotonic timestamps are never serialized.

## Testing Strategy

- Domain unit tests cover DST, offset fractions, state transitions, recurrence, pause/resume, laps, and missed deadlines.
- Application tests use fake clocks, scheduler, storage, and notification ports.
- Adapter contract tests verify settings migration and platform capability mapping.
- Slint component tests and golden images protect the three clock modes and ruler lens.
- A small end-to-end suite covers startup, tray recovery, persistence, and notification delivery.

## Architecture Constraints

- No domain decisions in `.slint` expressions or platform adapters.
- No direct filesystem or OS calls outside adapters.
- No duplicated timer loops or time-zone conversion implementations.
- No feature may depend directly on another feature’s UI.
- No new global mutable state. Keep application-owned `unsafe` out of domain and application crates; framework-generated UI code is isolated in the UI adapter.
- Cross-cutting decisions require an ADR; visible behavior requires requirement IDs and tests.
