# Repository Guidelines

## Project Structure & Module Organization

The active rewrite is a Rust workspace: `apps/oziclock-desktop/` is the executable, `crates/oziclock-domain/` owns framework-free rules, `crates/oziclock-app/` owns use cases, and `ui/` will contain Slint components. The preserved .NET 9/WPF reference remains in `legacy/dotnet-wpf/`, including its solution, release script, assets, and `Ozi.Clock/` source.

Before changing behavior, read `docs/AI_DEVELOPMENT_WORKFLOW.md`. Product behavior belongs in `docs/REQUIREMENTS.md`, target module boundaries in `docs/ARCHITECTURE.md`, future feature scope in `docs/FUTURE_FEATURES.md`, and architectural decisions in `docs/decisions/`. Ruler/lens rendering is specified in `docs/RULER_LENS_DESIGN.md`.

The required local tools and their installation boundaries are documented in `docs/DEVELOPMENT_ENVIRONMENT.md`. Do not add an IDE or runtime dependency merely for development convenience.

## Build, Run, and Development Commands

Run legacy commands from `legacy/dotnet-wpf/` on Windows with the .NET 9 SDK installed:

```powershell
dotnet restore Ozi.Utilities.sln
dotnet build Ozi.Utilities.sln -c Debug
dotnet run --project Ozi.Clock/Ozi.Clock.csproj
dotnet publish Ozi.Clock/Ozi.Clock.csproj -c Release -r win-x64
```

`restore` resolves dependencies, `build` compiles the solution, `run` launches the clock locally, and `publish` produces a Windows x64 release. `make_release.ps1` increments a version, creates archives, pushes a Git tag, and publishes a GitHub release; only maintainers should run it intentionally. When Rust is installed, use `cargo check --workspace`, `cargo test --workspace`, and `cargo run -p oziclock-desktop` from the repository root.

## Coding Style & Naming Conventions

For legacy C#, use four-space indentation and preserve existing XAML formatting. Nullable reference types are enabled and implicit usings are disabled. Follow `PascalCase` for public members, `camelCase` for locals, and `_camelCase` for private fields. Keep the existing WPF namespace and XAML/code-behind pairs unchanged unless preserving the reference requires a repair.

For the rewrite, use `rustfmt` defaults and keep `clippy` clean. Use `snake_case` for Rust modules and functions, `PascalCase` for types, and descriptive typed commands/events instead of string messages. Slint components use `PascalCase`; shared dimensions, colors, typography, and animation values belong in design tokens. Follow the dependency direction in `docs/ARCHITECTURE.md`; UI and domain code must not call operating-system or storage APIs directly.

## Testing Guidelines

The legacy project has no automated test project. Before modifying it, run `dotnet build Ozi.Utilities.sln -c Release` from `legacy/dotnet-wpf/` and manually exercise affected WPF flows. New Rust logic requires focused tests; visual changes require reviewed golden images.

Once the Rust workspace exists, every change must run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`. Time-based tests use fake UTC and monotonic clocks; visual changes update reviewed golden images rather than weakening comparisons.

## Commit & Pull Request Guidelines

History uses short, imperative, feature-focused subjects such as `implement feature "show/hide seconds"` and `fix time on showing slider`. Keep each commit scoped to one change and explain the user-visible outcome. Pull requests should include a concise summary, testing performed, and linked issue when applicable. Attach before/after screenshots for visual changes and call out configuration, packaging, or compatibility impacts.

## Configuration & Generated Files

Do not commit `bin/`, `obj/`, `publish/`, IDE settings, or user-specific `appsettings.*.json` files. Never place secrets in the checked-in base configuration.

## Development Workflow

After completing each user-requested task, build the Rust desktop application with `cargo build -p oziclock-desktop` and launch a debug instance with `cargo run -p oziclock-desktop` so the user can manually verify the result.
