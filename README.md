# OziClock

OziClock is being rebuilt as a lightweight, cross-platform desktop world clock. The target application will use Rust for application logic and Slint for the custom desktop UI.

## Repository Layout

- `apps/oziclock-desktop/` — future native desktop executable.
- `crates/` — reusable Rust domain and application modules.
- `ui/` — shared Slint components and design tokens.
- `docs/` — requirements, architecture, design specifications, and decisions.
- `legacy/dotnet-wpf/` — preserved WPF/.NET 9 reference implementation, assets, solution, and release script.

## Documentation

- [Product and architecture notes](docs/PRODUCT_ARCHITECTURE.md)
- [Target modular architecture](docs/ARCHITECTURE.md)
- [Functional and quality requirements](docs/REQUIREMENTS.md)
- [Ruler and magnifying-lens design](docs/RULER_LENS_DESIGN.md)
- [Future alarms, reminders, timers, and stopwatch](docs/FUTURE_FEATURES.md)
- [AI-assisted development workflow](docs/AI_DEVELOPMENT_WORKFLOW.md)
- [Minimal Windows development environment](docs/DEVELOPMENT_ENVIRONMENT.md)
- [Rust and Slint stack decision](docs/decisions/0002-rust-slint-stack.md)

The Rust toolchain is required before building the new workspace. The legacy application can still be built from `legacy/dotnet-wpf/` with the .NET 9 SDK.

## Current Prototype

The first Rust/Slint slice loads a dynamic clock list from portable JSON settings, renders reusable `ClockTile` components, and updates time once per second. On first launch it creates `settings.json` beside `oziclock-desktop.exe`. Edit this file to add, remove, reorder, recolor, or select the main time zone; use IANA names such as `Europe/Kyiv`.
