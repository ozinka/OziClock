# ADR 0007: One Fixed Per-User Settings Document

- Status: Accepted
- Date: 2026-09-25
- Requirements: SET-03, SET-11, SET-12; backlog BL-090

## Context

Custom local storage required a bootstrap next to the former default document,
an alternate recovery copy, and ongoing location selection. On Windows/Linux,
moving the executable could lose discovery of that bootstrap. The user chose
one predictable local document and removed the ability to relocate it in the UI.

## Decision

Use `%USERPROFILE%\.oziclock\settings.json` on Windows, the XDG configuration
directory's `oziclock/settings.json` on Linux (default `~/.config`), and
`~/Library/Application Support/OziClock/settings.json` on macOS. Windows uses
the explicitly selected home-directory convention instead of AppData. Ignore
relative or empty XDG values. Missing home-directory resolution is an error;
never silently write beside the executable.

Remove the local Storage controls and use Sync for the shared-profile section.
Keep device-only Sync metadata in the main document; only portable selected
groups belong in `oziclock-sync.json`. This updates ADR 0006's local storage
decision, without changing its shared-profile and conflict model.

Schema version 2 marks the one-time migration. A current fixed document wins.
For an unmigrated installation, prefer the valid custom file selected by the old
bootstrap, then its default recovery file, then the older macOS bundle-adjacent
file. macOS needs this migration even though its default directory is unchanged.
Import valid old Sync metadata, write the new document atomically, and only then
consider migration complete. Keep source files intact; do not maintain, update,
or consult them after successful migration. There is no separate marker file.

## Consequences and Validation

Updates and executable moves no longer change local settings discovery. Two
installations run by the same OS user share the fixed document. When upgrading,
legacy executable-adjacent data is discoverable only if the new binary initially
runs from the old installation directory; otherwise manual copying is needed.

Storage tests cover all path conventions, first launch, legacy/custom import,
macOS's same-directory migration, invalid/unavailable custom data, existing-file
precedence, embedded Sync persistence, and failed local saves. Validate the Sync
layout on a desktop build and retain reviewed visual references. Native macOS
and Linux runtime verification remains separate from host-independent tests.
