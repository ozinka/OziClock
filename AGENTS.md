# Repository Guidelines

## Collaboration and Efficiency

- Treat tentative wording such as "maybe" or "could we" as openness to alternatives, not a requirement to ask for confirmation. Separate the user's desired outcome from their suggested implementation; evaluate suggestions critically and explain material tradeoffs.
- Infer the intended action from the whole request. For discussion, evaluation, or diagnosis, inspect and explain without changing code. For implementation, proceed with routine decisions within the agreed scope. Clarify only unresolved choices that materially affect the outcome or authorization.
- Use existing requirements and the request to identify observable acceptance criteria. State assumptions briefly when useful; do not add a planning or approval round for straightforward work.
- Distinguish observations, hypotheses, and verified causes. For uncertain bugs, seek a discriminating reproduction or diagnostic before changing behavior. After two unsuccessful fix approaches to the same symptom, reassess the evidence and report what was ruled out before trying another approach; avoid stacking speculative workarounds.
- Keep related edits in one coherent change and avoid unrelated improvements. Read targeted files and relevant log excerpts. Reuse completed checks when the checked state has not changed; repeat or broaden them only for changed code, failures, or unresolved risks. Preserve all required validation and debug-launch rules below.
- After substantial investigation, record durable findings, rejected approaches and reasons, and unresolved questions in the appropriate existing project documentation. Keep this concise; do not create a report for every small task.
- Keep agreed requirements and decisions current within each feature. After every 3–5 completed features, reconcile documentation using `docs/AI_DEVELOPMENT_WORKFLOW.md` and persist the checkpoint in `docs/BACKLOG.md` so the cadence survives new chats.
- Use `docs/AI_REPOSITORY_GUIDE.md` as the task router and current ownership map. Load only the controlling requirements, backlog item, design, ADR, source, and tests needed for the requested slice.
- Finish with the outcome, verification, and any remaining limitation. Do not claim success from compilation alone when acceptance requires observing runtime or visual behavior.

## Project Structure & Module Organization

The active rewrite is a Rust workspace: `apps/oziclock-desktop/` is the executable, `crates/oziclock-domain/` owns framework-free rules, and `crates/oziclock-app/` owns use cases. Active window and feature Slint files live in `apps/oziclock-desktop/ui/`; `ui/` owns shared components, fonts, and future design tokens. The preserved .NET 9/WPF reference remains in `legacy/dotnet-wpf/`, including its solution, release script, assets, and `Ozi.Clock/` source.

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

`restore` resolves dependencies, `build` compiles the solution, `run` launches the clock locally, and `publish` produces a Windows x64 release. `make_release.ps1` increments a version, creates archives, pushes a Git tag, and publishes a GitHub release; only maintainers should run it intentionally. When Rust is installed, use `cargo check --workspace`, `cargo test --workspace`, and `cargo build -p oziclock-desktop` from the repository root. The user can launch the built debug binary with `target/debug/oziclock-desktop`.

## Coding Style & Naming Conventions

For legacy C#, use four-space indentation and preserve existing XAML formatting. Nullable reference types are enabled and implicit usings are disabled. Follow `PascalCase` for public members, `camelCase` for locals, and `_camelCase` for private fields. Keep the existing WPF namespace and XAML/code-behind pairs unchanged unless preserving the reference requires a repair.

For the rewrite, use `rustfmt` defaults and keep `clippy` clean. Use `snake_case` for Rust modules and functions, `PascalCase` for types, and descriptive typed commands/events instead of string messages. Slint components use `PascalCase`; shared dimensions, colors, typography, and animation values belong in design tokens. Follow the dependency direction in `docs/ARCHITECTURE.md`; UI and domain code must not call operating-system or storage APIs directly.

## Testing Guidelines

The legacy project has no automated test project. Before modifying it, run `dotnet build Ozi.Utilities.sln -c Release` from `legacy/dotnet-wpf/` and manually exercise affected WPF flows. New Rust logic requires focused tests; visual changes require reviewed golden images.

Once the Rust workspace exists, every change must run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`. Time-based tests use fake UTC and monotonic clocks; visual changes update reviewed golden images rather than weakening comparisons.

Documentation changes must run `sh scripts/check-docs.sh`. Documentation-only changes do not require a Rust build or debug launch. New focused test names should include the normalized controlling requirement ID when practical, such as `alm_11_due_interval_is_half_open`.

## Commit & Pull Request Guidelines

History uses short, imperative, feature-focused subjects such as `implement feature "show/hide seconds"` and `fix time on showing slider`. Keep each commit scoped to one change and explain the user-visible outcome. Pull requests should include a concise summary, testing performed, and linked issue when applicable. Attach before/after screenshots for visual changes and call out configuration, packaging, or compatibility impacts.

## Configuration & Generated Files

Do not commit `bin/`, `obj/`, `publish/`, IDE settings, or user-specific `appsettings.*.json` files. Never place secrets in the checked-in base configuration.

## Development Workflow

After every source-code change, run the required checks and launch the debug build for the user to verify. On macOS, run `sh scripts/launch-debug-macos.sh`; it builds the application, creates or updates a temporary `.app` bundle, and opens it through Launch Services without Terminal. If a running instance prevents a rebuild, ask the user to close it, then retry.

Run build commands from the repository root. Prefer debug builds for quick local verification. Build and validation are required after every code change, including changes made while fixing a failed build or release.
