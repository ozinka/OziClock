# ADR 0006: Versioned Sync Profile in a Provider-Managed Folder

- Status: Accepted
- Date: 2026-09-23
- Requirements: SET-11; backlog BL-018

## Context

OziClock currently persists all data in one local `settings.json`. The user
wants a Mac and Windows installation to share selected data through a folder
already synchronized by Google Drive, OneDrive, Dropbox, or a similar desktop
client. They need continuous bidirectional synchronization and explicit
one-time Send and Receive actions. Device-specific preferences must remain
local.

Copying a complete local settings document to a shared folder would propagate
window geometry, launch-at-login and platform state, and concurrent edits could
silently overwrite data or produce provider conflict copies. Moving a SQLite
database into a synchronized folder has the same conflict problem and adds
database sidecar files without defining a merge policy.

## Decision

Keep the versioned JSON local store. Add one versioned `oziclock-sync.json`
profile in a user-selected provider-managed folder. It contains only explicitly
enabled portable groups: Planner data, Clocks, and Appearance. It excludes
device settings, system permissions, active timing/delivery state and all other
local-only data.

Each installation keeps a local-only `Sync` section in its fixed `settings.json`
with its profile path, enabled groups, last common revision and unresolved conflict state.
ADR 0007 replaces the original separate `sync-state.json` with this section. The path is
not synchronized because it differs across operating systems. A device can
continuously synchronize in both directions or issue previewed one-time Send
to profile and Receive from profile actions.

The profile uses stable IDs, per-field revisions and deletion tombstones. Merge
independent changes automatically. Present a user decision when both sides
changed the same field, or a deletion conflicts with an edit. Never let profile
I/O delay local saves, scheduling or alerts. The provider desktop client moves
the profile; the first slice does not require provider authentication or an API
integration.

## Consequences and Validation

The Sync profile is inspectable and easy to back up. The implementation needs a
schema migration strategy, atomic profile replacement, local recovery from
malformed/unavailable files, and clear status for provider-created conflict
copies. It must preserve profile groups that the current device has not enabled.
It also needs tests for Mac/Windows path differences, one-time directions,
continuous offline recovery, category filtering, concurrent entity/field edits,
delete conflicts and no duplicate local deliveries.

SQLite remains an option for a future local store if query volume or history
requires it; it is not stored directly in a synchronized folder and does not
replace the Sync profile merge model.
