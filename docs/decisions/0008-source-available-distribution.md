# ADR 0008: Source-Available Distribution for the Active Rewrite

- Status: Accepted
- Date: 2026-09-26
- Backlog: BL-099

## Context

OziClock should remain available without charge, including for internal use in
commercial organizations. The maintainer does not grant permission for a third
party to sell OziClock itself or a minimally changed fork as its own product.

The active Rust/Slint rewrite is authored independently of the preserved WPF/
.NET implementation. A former contributor participated only in that legacy
implementation, which must retain its existing MIT terms.

## Decision

License the active Rust/Slint rewrite under the MIT License with Commons Clause
License Condition v1.0. The condition preserves ordinary MIT rights, including
corporate and commercial use, while excluding the right to sell software whose
value derives entirely or substantially from OziClock's functionality.

Describe the rewrite as source-available, not open source in the OSI sense.
Keep `legacy/dotnet-wpf/` under a separate unmodified MIT License. Cargo
metadata points to the root license file instead of declaring the incomplete
SPDX identifier `MIT`.

## Consequences and Validation

Future releases of the active rewrite carry the combined license and must
retain both the MIT and Commons Clause notices. Existing MIT releases remain
MIT-licensed; the new condition cannot revoke the permissions already granted
for them.

Documentation review confirms that the repository communicates the scope of
both licenses and that Cargo points to the combined license text. The decision
does not establish a donation platform, store-publication commitment, or a
trademark policy; those remain separate decisions.
