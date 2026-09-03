# Personal Time Tools — Discovery Design

Status: proposed. This document defines the first product slice that can move the
alarms, timers, stopwatch, reminders, and tasks backlog items to `Ready`.

## Product Shape

The clock strip remains the primary OziClock surface. Personal time tools live
in one optional, frameless **Planner** panel opened from the tray or clock-strip
context menu. It restores the last selected tool and does not turn the strip
into a dashboard. The panel has five persistent destinations:

- **Alarms** — a collection of independent one-time or weekly alarms.
- **Timers** — named countdowns that can run concurrently.
- **Stopwatch** — one local stopwatch with laps.
- **Reminders** — text notifications scheduled at an instant or after a delay.
- **Tasks** — a simple actionable list with an optional due date/time and an
  optional reminder. It is not a calendar or a project-management system.

### Task, reminder, and important-date model

A **task** is an action with a lifecycle (`open`, `completed`, or `cancelled`).
It may have an optional deadline, optional scheduled start/end time, optional
custom color, and zero or more alert rules. A deadline means “finish by this
moment”; a scheduled time means “work on it at this moment.” Either one may be
absent and they are never conflated.

A **reminder** is a notification rule that does not require completion. It may
stand alone (“Water the plants every Sunday”) or be attached to a task. A task
therefore remains one task even when it has alerts such as one day, 30 minutes,
and 15 minutes before its deadline. Every alert occurrence has its own delivery
receipt to prevent duplicate notification after recovery.

An **important date** is an all-day, usually yearly reminder for a fact such as
a birthday or anniversary. It has no completion state. A user may create a
separate task (“Buy Anna a gift”) that has its own deadline and alerts. The UI
groups important dates under Reminders while keeping their all-day semantics.

Planner is visually derived from the main clock, just as Calendar is. The
desktop adapter supplies the selected main clock's accent and configured corner
radius as design tokens. Planner derives accessible surface, border, hover, and
selection variants from that accent; it does not persist a separate Planner
theme. The outer panel uses the clock radius plus the same small panel offset
as Calendar, while controls and item surfaces use proportionally smaller values.
Changing the main clock's color or radius updates both Calendar and Planner
while open.

Tasks may optionally have a custom color. It is stored on the task as an
accessible color value and defaults to the main-clock accent when unset. In
Planner it colors only the task's list edge, Month marker, and Week block edge;
the text and panel surfaces retain the shared theme for contrast. A task color
does not alter clock-strip indicators, alarms, reminders, or the global
Calendar/Planner theme.

Each destination has a focused list and one obvious creation action. Editing
uses a compact detail sheet, rather than inline controls that move as items
change state. The HTML prototype in `reminders-tools-prototype.html` explores
these list and detail-sheet interactions.

### Planner calendar

The Planner opens to Week and orders its views as Week, Month, then Year. It reuses the calendar
module's date-selection, navigation, locale, and first-day-of-week rules, but
does not inherit the transient calendar panel's focus-loss behavior. It can be
kept open while a user plans.

The transient calendar panel remains a fast date viewer. It shows compact
markers for scheduled tasks, reminders, and due dates. Activating a marker
opens Planner at the relevant date and view: a timed task opens Week, an
all-day or due item opens Month, and a date-only marker opens Month with that
date selected. The Planner is also reachable from the tray/context menu as
`Open Planner — This week`.

Only scheduled tasks occupy a time block. A due date appears as a deadline
marker, not as a block; a task may have both. Unscheduled tasks remain in the
task list and in an explicit Planner count. Reminders show as narrow timed
markers, while alarms, timers, and stopwatch sessions do not become calendar
events.

### Today indicators on the clock strip

The primary clock tile may show up to three small, neutral indicators when
there is something actionable today: alarm, task, and reminder/important-date.
Each icon carries a count only when it is greater than one. The indicators are
not a second agenda, do not animate, and are absent when the count is zero.
Selecting one opens the matching Planner list filtered to today; an optional
combined indicator opens Planner Week. A finished timer temporarily replaces
its own active-tool chip with an explicit finished state, rather than adding a
fourth indicator. The UI must keep the clock's time readable at every scale.

### Active timer and stopwatch shelf

Starting a timer or stopwatch in Planner remains effective after Planner is
closed. While one is active, a narrow **activity shelf** attaches beneath the
clock strip and inherits its accent and corner radius. It shows a tool icon,
name, and live value. Selecting it opens the corresponding Planner detail;
hover/context controls provide pause, reset, and stop where supported.

The shelf shows at most the nearest-finish timer and the stopwatch. Additional
active timers collapse into a `+N` control that opens Timers. At constrained
width, names disappear before live values. On completion, the timer's shelf
entry briefly becomes “finished” with `Dismiss` and `Restart`; native
notification and the item sound policy handle the alert itself. Dismissing it
removes the entry.

## Scheduling Semantics

All notifications are scheduled by one application scheduler. It is the only
component allowed to request the next deadline; features must not start polling
threads.

| Feature | Source of truth | Restart behavior | When overdue after sleep/restart |
| --- | --- | --- | --- |
| Alarm | Local wall time, IANA zone, recurrence | Enabled alarms are reconstructed and rescheduled | Deliver once if the occurrence is within the configured grace period; otherwise record it as missed and calculate the next occurrence. |
| Reminder | Absolute UTC instant or relative delay resolved to UTC when saved | Pending reminders are reconstructed | Deliver once; a delivery receipt prevents duplicates. |
| Task | Optional due instant and optional linked reminder | Tasks remain until completed or deleted | Mark due and deliver only its linked reminder according to its receipt. |
| Timer | Duration plus started/paused UTC timestamps | Running timers derive remaining duration from timestamps; paused timers preserve remaining duration | Finish immediately if elapsed; a repeating timer creates its next interval only after the completion is reconciled. |
| Stopwatch | Monotonic elapsed duration while running; lap list | Persist only idle/paused state in v1 | A running stopwatch is paused at recovery and labelled as interrupted. |

For an alarm whose requested local time does not exist during a DST jump,
schedule it at the first valid local time after the gap. For an ambiguous local
time during the backward jump, trigger once at the earlier occurrence. These
policies are visible in help text and must be shared by recurrence calculation
and notification delivery.

### Alarm recurrence

An alarm is either `once` (a date, local time, and zone) or `weekly` (one or
more weekdays, local time, and zone). An empty weekday selection is invalid.
Changing its zone preserves the displayed wall-clock time; it deliberately does
not preserve the old instant. Snooze is transient: it replaces the next
delivery only and never changes the recurrence rule. Dismiss ends the current
delivery and finds the next occurrence. There is no automatic recurring snooze.

### Reminder and task recurrence

Reminders support `none`, daily, weekly, monthly, yearly, and a simple custom
interval. A recurrence is calculated from the scheduled local date/time and its
zone, then stored as the next UTC instant. Tasks do not repeat in the first
slice: a repeated responsibility is modelled as a recurring reminder until a
future task-series design is justified.

The recurrence editor offers structured patterns instead of free-form text:

- every `N` days;
- every `N` weeks on selected weekdays, anchored to an explicit first date;
- monthly on a numbered day, the first day, or the last day;
- monthly on the first, second, third, fourth, or last selected weekday (for
  example, “the last Friday”);
- yearly on a date, including all-day important dates such as birthdays.

The interval is stored as typed rule fields, such as
`weekly { interval: 2, weekdays: [Monday, Thursday], anchor_date }` or
`monthly_by_weekday { ordinal: Last, weekday: Friday }`. “Every second/third
week” always means a whole-week interval from the anchor date; it must not mean
the second or third week of a month. The preview in the editor shows the next
three occurrences before saving.

## Storage Recommendation

Use the existing `oziclock-storage` versioned JSON application document for
v1. Add a `Planner` section with stable UUIDs, IANA zone identifiers, explicit
format version, and per-feature arrays. Keep every timestamp as RFC 3339 UTC;
only alarm/reminder recurrence also stores its local time and zone. Save by
writing a temporary sibling file, syncing it, then atomically replacing the
previous document. The current direct write must be upgraded before planner
data is persisted.

JSON is preferable now because the expected collection is small, one process
owns it, it remains easy to inspect/back up, and the repository already has a
versioned migration path. SQLite becomes appropriate only when we need indexed
history, large event logs, transactional multi-device merge metadata, or richer
task/calendar queries. Selecting SQLite early would add migration, packaging,
backup, and inspection cost without helping the first local-only slice.

The storage port should expose typed load/save operations, not JSON values. A
future sync adapter maintains a provider mapping such as
`{ local_id, provider, remote_id, remote_revision, last_synced_at }` outside
the domain objects. Start Google Calendar and Outlook integration as
one-way import with an explicit account connection and conflict review; do not
silently create or edit external events. Alarms, timers, and stopwatch stay
local-only. Calendar events may be surfaced as reminders later, but external
sync must never become a prerequisite for offline scheduling.

### Import and export

Planner provides a user-initiated, portable **Planner archive** export in a
versioned UTF-8 JSON document (suggested extension
`.oziclock-planner.json`). It contains Planner entities, stable IDs, recurrence
rules, task colors, and future scheduling state. It excludes operating-system
notification permission, local window state, sound file paths, account tokens,
and platform-specific settings. Exporting Planner data must not overwrite or
expose the user's world-clock configuration by default.

Import always validates the schema and previews counts, unsupported fields,
overdue items, and conflicts before writing anything. It offers two explicit
modes:

- **Merge** (default): add items with new IDs; for an existing stable ID, show
  the user whether to keep the local item, keep the imported item, or retain
  both as separate copies.
- **Restore**: replace the complete Planner section only after creating a
  local backup and receiving destructive-action confirmation.

An import never sends notifications immediately for historical occurrences.
It recalculates each next occurrence and records any overdue state for review
in Planner. This prevents an imported archive from producing a burst of old
alerts. A full Planner archive is the v1 interchange and backup format.
Import/export of standard iCalendar (`.ics`) is a later compatibility slice:
initially export scheduled tasks and important dates as calendar events, then
offer previewed one-way `.ics` import. Outlook and Google Calendar connections
remain separate, opt-in sync adapters rather than extensions of archive import.

### Local recovery backups

The storage adapter keeps a rolling local backup set of the five most recent
calendar days. After the first successful Planner save on a local day, it writes
an immutable, validated Planner archive to the per-user backup directory using
the same atomic-write protocol as the primary document. A later save on that
day replaces that day's backup only after it is completely written and
validated. After a successful new daily backup, the adapter prunes backups
older than the five newest distinct local dates.

Before an import, restore, or schema migration, the adapter creates a recovery
snapshot first; it may replace the current day's backup so the retained set
never exceeds five user-visible copies. Restore offers these dated backups in
addition to a manually chosen archive. Backup creation or pruning failure must
not corrupt the primary document: the operation reports the issue, preserves
the current document, and never deletes the only valid copy. Backups are local
and independent of cloud synchronization.

### Optional encrypted Planner storage

Password encryption is a planned, opt-in storage capability, not a domain
feature. Domain and application use plain typed Planner values; an
`EncryptionProvider` sits between serialization and the local/archive/cloud
adapters. The persisted format is an encrypted envelope with a format version,
algorithm identifier, random salt, KDF parameters, nonce, ciphertext, and
authentication tag. It contains no password, derived key, or plaintext preview.

The implementation must use a reviewed memory-hard password KDF and an
authenticated-encryption algorithm supplied by maintained Rust crypto crates;
it must never invent cryptography. Password material is accepted only for
unlocking, is never written to settings, logs, exports, or crash reports, and
is cleared from memory as promptly as the platform permits. Changing a password
requires the old password and re-encrypts the primary document, retained
backups, and new exports. There is intentionally no password recovery path.

When encryption is enabled, automatic daily backups and Planner archives are
encrypted as well. Cloud providers store only the encrypted envelope; a device
downloads, authenticates, and decrypts locally before merging entity changes,
then encrypts the merged document before conditional upload. Remote revision
metadata may remain visible but content does not.

The user must choose one of two explicit startup policies: unlock Planner after
each app launch (Planner notifications cannot be scheduled until unlocked), or
`Remember on this device`, which stores a device-bound unlock secret in the OS
credential store so local scheduling can resume after login. Locking hides
Planner content and clears the active key; it does not leave decrypted copies in
the export or backup folders. Enabling encryption migrates a validated plaintext
document only after creating a recovery backup and confirming that future
archives will require the password.

### Custom location and multi-device sync

A user may choose a custom folder, including a folder managed by the installed
Google Drive or OneDrive desktop client. It has two explicit modes. **Backup
export** writes complete Planner archives atomically to timestamped files for
manual transfer and recovery. **Folder sync** is the initial multi-device mode:
it uses that same provider-managed folder but never treats one shared JSON file
as the live database.

Folder sync writes immutable, atomically-created encrypted-or-plain change
records named by device ID and operation ID. Each device applies unseen records
idempotently to its local Planner state, merges them by entity version and
tombstone rules, then creates its own next record. Concurrent devices therefore
add distinct files instead of overwriting a shared document; no cross-computer
filesystem lock is required. The sync client may deliver records late, so
OziClock scans on startup, after local changes, periodically, and through
`Sync now`. Its UI distinguishes local save, folder scan, and the sync client's
last observed remote change; it never promises instant delivery.

Checkpoint compaction is a later optimization and may not remove immutable
records until every known device has acknowledged them. Local recovery backups
remain a separate rolling set and are never used as merge input.

Direct provider synchronization is a later opt-in feature behind the same
`SyncStore` port. It uses an authenticated, app-owned remote document and a
provider revision token, rather than assuming a local synchronized folder is a
database. The local application document remains available offline and is the
only store the scheduler reads while running.

Each Planner entity has a stable ID, device ID, modification version, and a
deletion tombstone. On sync the adapter reads the current remote revision,
merges independently changed entities, and writes only if the remote revision
is unchanged. On a competing write it reads again, merges, and retries. This
avoids lost updates without relying on filesystem locks, which cloud-sync
clients cannot make reliable across computers. Identical IDs merge by the most
recent explicit field change; an irreconcilable concurrent edit is retained as
a visible conflict copy for user review. Deletes win only when they are newer
than the edited entity; tombstones are retained until every known device has
acknowledged them.

The first folder-sync slice provides `Sync now`, visible status, conflict
review, and periodic folder scans. Direct OAuth provider sync, shared/team
planners, and arbitrary editing of generated sync files are later work.
Authentication tokens for that later direct-provider mode belong in the
operating-system credential store, never in the Planner archive or settings
JSON.

## Notification Policy

The application requests native notification permission only when the first
notification-capable item is enabled. A notification contains the item title,
time, and actions appropriate to the feature: snooze/dismiss for alarms and
complete/snooze for reminders or tasks. Sound is a per-item policy with a
global default. A durable delivery receipt `(item_id, occurrence_id)` is saved
before notifying so restart cannot duplicate an alert. Notification history is
retained for 30 days, then pruned by the storage adapter.

### Attention and focus policy

Planner never opens or takes focus automatically. Event delivery has three
layers: a native operating-system notification; a local attention surface when
the clock strip is visible; and a durable `Needs attention` state in Planner.
Selecting a notification or local surface opens Planner on that exact item.

An alarm is the strongest case: its configured sound plays and a compact,
frameless alarm card attaches to the clock strip with `Snooze` and `Dismiss`.
It persists until acted upon but does not activate the Planner window. A
finished timer posts its notification and changes its activity-shelf entry to
`Finished`, with `Dismiss` and `Restart`. A reminder or task alert uses a
temporary attention card with `Snooze`/`Dismiss` or `Complete`/`Snooze`;
the unresolved item remains in Planner afterwards. Important dates use a quiet
notification by default, without a local card or sound.

If native notification permission is unavailable, the local attention surface
and Planner state remain the fallback. System focus modes are respected; the
application does not attempt to bypass them.

## Prototype Evidence

`reminders-tools-prototype.html` is the agreed interaction study. It depicts
the Planner navigation; Week, Month, and Year planning views; task colors;
today indicators; and the active-tool shelf attached to a representative clock
strip. It is a visual contract for the first implementation, not production
UI code.

## Suggested Vertical Slices

1. `BL-007`: one-time and weekly alarms, local persistence, scheduler,
   notification delivery, dismiss and snooze.
2. `BL-009`: multiple countdown timers with pause/restart recovery.
3. `BL-010`: local stopwatch with laps and explicit running-session recovery.
4. `BL-008`: absolute and repeating reminders with delivery receipts.
5. Add tasks with an optional linked reminder after the reminder model is
   stable.

Before the first implementation, add requirement IDs and focused tests for DST
gaps/overlaps, sleep recovery, duplicate prevention, and atomic-save recovery.
