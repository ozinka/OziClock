# AI-Assisted Development Workflow

## Sources of Truth

Use documentation in this order:

1. `REQUIREMENTS.md` defines observable behavior and acceptance criteria.
2. Feature design specifications define geometry and interaction details.
3. `ARCHITECTURE.md` defines module boundaries and dependency rules.
4. ADRs explain significant technical choices.
5. The WPF application and screenshots are legacy evidence, not the target architecture.

`BACKLOG.md` is the canonical work queue. It tracks priority and status but does not override requirements or feature designs.

When code and documentation disagree, do not silently copy current behavior. Identify the conflict and update the appropriate source of truth with the implementation.

## Change Workflow

1. Locate or add requirement IDs for the requested behavior.
2. Reference the stable backlog ID and set the item to `In Progress` when work begins.
3. Identify the owning domain, application, UI, and adapter modules.
4. Reuse or extend shared components before creating a new one.
5. Add an ADR only for long-lived, cross-cutting, or difficult-to-reverse decisions.
6. Implement the smallest vertical slice without bypassing module boundaries.
7. Test domain behavior independently from UI and OS integration.
8. Run formatting, linting, unit tests, golden tests, and relevant platform smoke tests.
9. Update requirements, design notes, and backlog status in the same change.

## User Confirmation and Build Recovery

When the user requests an implementation, proceed immediately if the scope is clear. Ask for confirmation only when a choice would materially change product behavior, design, data, or external state. State the interpretation and present a single concise confirmation question.

After every implementation change, run the required build and validation commands. If a build cannot overwrite the executable because the application is running, do not stop at a status report: ask the user to close the application and confirm continuation. The user reply `1` means “Ready, continue”; retry the build immediately without requesting further clarification.

### Manual debug launch

For the Rust desktop application, run commands from the repository root:

```bash
cargo build -p oziclock-desktop
cargo run -p oziclock-desktop
```

`cargo run` builds (when needed) and starts the local debug binary in the foreground. When commands run through an agent shell, the process may belong to an isolated session and its GUI window may not be visible in the user's macOS session. In that case, the user must run the same command in their own Terminal. Keep a visible instance running while the user tests; stop it with `Ctrl-C` when a rebuild is required. After every code change, repeat both commands and explicitly report whether the agent could verify a visible window.

Use the conversation language selected by the user for all messages to the user. Keep code, comments, UI copy, documentation, commit messages, and runtime diagnostics in English only.

## Definition of Done

A feature is complete only when its behavior is documented, module ownership is clear, failure and restart cases are handled, tests cover its acceptance criteria, UI matches design tokens, resource impact is measured when relevant, and no known platform limitation is hidden. Temporary shortcuts must be recorded explicitly; comments are not substitutes for tracked architectural decisions.

## Implementation Discipline

Prefer typed models over dictionaries and stringly typed messages. Keep functions small around meaningful use cases, but do not split code solely to increase file count. Do not duplicate constants, time calculations, serialization logic, or platform checks. Reject point fixes that create a second source of truth; repair the owning abstraction instead.
