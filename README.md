# OziClock

OziClock is a lightweight desktop world clock for viewing several time zones at once. Version 2 is a native Rust and Slint rewrite of the original WPF/.NET application.

## Downloads

Download the current release from [GitHub Releases](https://github.com/ozinka/OziClock/releases/latest).

- **Windows x64:** unzip and run `oziclock-desktop.exe`. No .NET or Visual C++ Redistributable installation is required.
- **Linux x64:** extract the archive, mark `oziclock-desktop` executable if necessary, then run it.
- **macOS Apple Silicon:** extract the archive and run `oziclock-desktop` from Terminal.

The application creates `settings.json` beside the executable on first launch. Keep that file next to the executable to preserve clocks, placement, and preferences.

The last legacy WPF/.NET release is [v1.0.10](https://github.com/ozinka/OziClock/releases/tag/v1.0.10). Its source is preserved in [`legacy/dotnet-wpf/`](legacy/dotnet-wpf/).

## Repository Layout

- `apps/oziclock-desktop/` — native Rust desktop executable.
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

The Rust toolchain is required to build the active workspace. The legacy application can still be built from `legacy/dotnet-wpf/` with the .NET 9 SDK.

## Features

- Multiple configurable world clocks using IANA time zones.
- Frameless, draggable, always-on-top clock strip with compact mode.
- Portable `settings.json` storage and configurable clock scale.
- Tray controls, contextual actions, and custom Settings/About windows.
- Extended ruler mode with an interactive Lens and Column slider for time exploration.

## Build from Source

Install the Rust stable toolchain, then run from the repository root:

```powershell
cargo run -p oziclock-desktop
cargo build --release -p oziclock-desktop
```

Windows builds use a statically linked Visual C++ runtime, so the release executable does not depend on `VCRUNTIME140.dll`.
