# AI Repository Guide

This is the shortest reliable entry point for an AI agent working on OziClock. It maps the current repository, identifies the source of truth for each kind of decision, and defines the evidence expected from a change. Detailed rules remain in the linked documents.

## Read by Task

Always read `AGENTS.md`. Then load only the documents that control the requested change:

| Task | Required context |
| --- | --- |
| Observable behavior or bug | `REQUIREMENTS.md`, the matching `BACKLOG.md` item, and the owning source/test module |
| Module boundary or dependency | `ARCHITECTURE.md` and relevant ADRs in `decisions/` |
| Ruler or lens rendering | `RULER_LENS_DESIGN.md` and `VISUAL_TESTING.md` |
| Planner reminder discovery | `REMINDERS_DESIGN.md`; confirm approved behavior in `REQUIREMENTS.md` before implementation |
| Build, packaging, or platform setup | `DEVELOPMENT_ENVIRONMENT.md`, workflow files, and the relevant platform adapter |
| Future product idea | `FUTURE_FEATURES.md`; do not treat it as approved behavior |

If sources disagree, follow the precedence in `AI_DEVELOPMENT_WORKFLOW.md` and report the conflict. Existing code and the legacy WPF application are evidence, not automatic approval.

## Current Ownership Map

The current implementation is intentionally less split than the target architecture:

| Path | Current responsibility |
| --- | --- |
| `crates/oziclock-domain/` | Framework-free entities, invariants, and state transitions |
| `crates/oziclock-app/` | Use cases, scheduling calculations, typed commands, and application orchestration |
| `crates/oziclock-storage/` | Versioned JSON settings, migration, and atomic persistence |
| `apps/oziclock-desktop/src/desktop/` | Composition root plus Slint, window, notification, audio, and operating-system adapters |
| `apps/oziclock-desktop/ui/` | Active application windows and feature-specific Slint components |
| `ui/` | Shared Slint components, fonts, and future design tokens |
| `legacy/dotnet-wpf/` | Preserved behavioral and visual reference only |

Do not create a crate merely to match the target diagram. Extract a boundary when it owns independently testable behavior or isolates an external system. Keep new framework-free behavior out of `desktop/mod.rs` even when callback wiring remains there.

## Change Contract

For each implementation change:

1. Name the controlling requirement IDs and backlog ID before editing. Add missing approved behavior to `REQUIREMENTS.md`; do not promote tentative ideas silently.
2. Identify the owning layer and the cheapest check that can falsify the proposed change.
3. Preserve save-before-display semantics for persisted Planner mutations and deterministic time inputs for scheduling tests.
4. Add focused tests. New test names should begin with a normalized requirement ID when one test directly proves that requirement, for example `alm_11_due_interval_is_half_open`.
5. Record manual or platform evidence in the backlog item when automation cannot prove an acceptance criterion.
6. Run the validation matrix below and update requirements, decisions, and backlog status in the same change.

Do not put implementation progress in source TODO comments. Keep unresolved product choices and verification gaps in `BACKLOG.md`; use an ADR for accepted, cross-cutting, difficult-to-reverse decisions.

## Validation Matrix

| Changed surface | Required validation |
| --- | --- |
| Documentation only | `sh scripts/check-docs.sh` |
| Rust or Slint source | `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; `cargo build -p oziclock-desktop` |
| macOS application behavior | Source checks above, then `sh scripts/launch-debug-macos.sh` and user-visible verification |
| Visual behavior | Source checks, debug launch, and the relevant checklist in `VISUAL_TESTING.md` or the feature design |
| Release or packaging | Source checks plus the affected release workflow or documented platform packaging check |

Successful compilation does not prove runtime, persistence, notification, audio, window-placement, or visual acceptance criteria. State unverified criteria explicitly.

## Documentation Contracts

Requirement IDs are stable and unique. Use a letter suffix for a compatible refinement rather than renumbering established requirements. Backlog IDs are also stable and may appear in either the active queue or completed archive, never both.

`scripts/check-docs.sh` verifies unique requirement and backlog definitions and rejects references to undefined IDs from the backlog and ADRs. CI and release quality jobs run this check. When adding a new ID family, update the validator in the same change.

ADRs use `Proposed`, `Accepted`, `Superseded`, or `Rejected`. A `Proposed` ADR is not authoritative until its validation gate is met or the status and remaining limitations are explicitly reconciled.
