# ADR 0002: Rust and Slint Application Stack

- Status: Proposed
- Date: 2026-08-23
- Supersedes: ADR 0001

## Context

OziClock must be cross-platform, visually precise, simple to evolve through AI-assisted development, and substantially lighter than the WPF/.NET implementation. Flutter offers productive UI development but includes a larger managed runtime and rendering engine than desired for this small utility.

## Decision

Use Rust for domain, application, persistence, and platform logic. Use Slint for declarative UI. Start the ruler prototype with Winit + FemtoVG, explicitly disabling Slint default renderer features. Retain Skia as a renderer fallback if measurement requires it. Isolate tray, notifications, always-on-top, frameless-window behavior, and taskbar/dock integration behind platform ports.

Adopt a Cargo workspace with inward dependency direction as specified in `docs/ARCHITECTURE.md`. Use a single scheduler and injectable UTC and monotonic clocks for all time-based features.

## Validation Gate

The decision becomes Accepted only after a Windows prototype demonstrates the eight-tile strip, compact and standard modes, ruler lens at 60 FPS, tray recovery, transparent always-on-top behavior, correct DPI scaling, and materially improved release-build memory and idle CPU use. macOS and Linux platform spikes must validate window and tray capabilities before the legacy app is retired.

## Consequences

Slint markup provides reusable declarative components while Rust keeps runtime overhead low and domain logic testable. Some desktop integration requires platform-specific Rust and Linux tray dependencies. Slint licensing requires choosing GPLv3 or satisfying the royalty-free community attribution terms. Renderer choice remains measurement-driven.
