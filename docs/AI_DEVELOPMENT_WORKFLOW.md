# AI-Assisted Development Workflow

## Sources of Truth

Start with `AI_REPOSITORY_GUIDE.md` for the current ownership map and task-specific reading paths. It is an index, not a source of product behavior.

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

New focused test names should begin with the normalized controlling requirement ID when practical, for example `alm_11_due_interval_is_half_open`. For criteria that require visual, hardware, or platform verification, record the exact pending or completed evidence in the backlog item rather than implying that an automated test covers it.

## Requirements and Decision Reconciliation

Record agreed observable behavior in the relevant functional requirements (FRs) during the feature, future requests in the backlog, and significant architectural decisions in ADRs under the existing ADR criteria. Tentative suggestions and unresolved alternatives are not approved requirements; label them as open questions or candidate work. Do not defer these updates until the periodic review.

After 3 completed features, perform a short documentation reconciliation at a natural task boundary, and no later than the fifth completed feature. Count distinct backlog items reaching `Done`, not commits, chat turns, or intermediate edits; do not mark a feature complete while required verification is pending.

Review the affected FRs, design notes, decisions, and backlog against implemented and verified behavior. Update existing entries rather than duplicating them, preserve stable IDs, and flag unresolved discrepancies instead of treating implementation as approval. Keep the review scoped to changes since the checkpoint; it is not a full repository audit.

Maintain the `Documentation Reconciliation` checkpoint in `BACKLOG.md`: the last review date and covered backlog IDs, distinct feature IDs completed since that review, and any unresolved follow-ups. Update the pending IDs whenever a feature reaches `Done`. After reconciliation, record the covered IDs and clear the pending list. If no checkpoint exists, establish an explicitly unreviewed baseline without claiming a retrospective audit.

## User Confirmation and Build Recovery

When the user requests an implementation, proceed immediately if the scope is clear. Ask for confirmation only when a choice would materially change product behavior, design, data, or external state. State the interpretation and present a single concise confirmation question.

After every implementation change, run the required build and validation commands. If a build cannot overwrite the executable because the application is running, do not stop at a status report: ask the user to close the application and confirm continuation. The user reply `1` means “Ready, continue”; retry the build immediately without requesting further clarification.

### Build and debug launch

After changes, run the required checks and build the Rust desktop application from the repository root:

```bash
cargo build -p oziclock-desktop
```

After every code change, launch the debug build for user verification. On macOS, run the following command from the repository root:

```bash
sh scripts/launch-debug-macos.sh
```

The script builds the debug binary, creates or updates `/private/tmp/OziClock-Debug.app`, links its executable to `target/debug/oziclock-desktop`, and opens the bundle through Launch Services without opening Terminal. If a release build was requested, build and launch the release binary instead.

Use the conversation language selected by the user for all messages to the user. Keep code, comments, UI copy, documentation, commit messages, and runtime diagnostics in English only.

## Definition of Done

A feature is complete only when its behavior is documented, module ownership is clear, failure and restart cases are handled, tests cover its acceptance criteria, UI matches design tokens, resource impact is measured when relevant, and no known platform limitation is hidden. Temporary shortcuts must be recorded explicitly; comments are not substitutes for tracked architectural decisions.

Run `sh scripts/check-docs.sh` after changing requirements, backlog entries, ADRs, or AI-facing documentation. Documentation-only changes do not require building or launching the application. Source changes still require the build, validation, and debug-launch workflow above.

## Implementation Discipline

Prefer typed models over dictionaries and stringly typed messages. Keep functions small around meaningful use cases, but do not split code solely to increase file count. Do not duplicate constants, time calculations, serialization logic, or platform checks. Reject point fixes that create a second source of truth; repair the owning abstraction instead.
