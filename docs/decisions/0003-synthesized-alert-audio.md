# ADR 0003: Shared Synthesized Alert Audio

- Status: Accepted
- Date: 2026-09-07
- Requirements: NTF-01; backlog BL-042

## Context

Platform-specific sound calls produced inconsistent behavior. macOS playback through `afplay` was reported silent; the adapter only observed process spawning, so the underlying failure is unconfirmed. The Windows beta build failed because `MessageBeep` used the wrong bindings module. Application size remains a priority.

## Decision

Use Rodio 0.22.2 with default features disabled and only `playback` enabled. Generate a 0.8-second two-tone chime in memory with a smooth envelope. Do not bundle sound files, codecs or recording support. Keep the implementation inside the desktop adapter and leave native notifications independent.

The composition root owns a clonable audio sender, passed explicitly to each Planner scheduler. General Settings persist one duration for all Planner alerts: Off, 1, 5, 10, 20 or 30 seconds, with 20 seconds as the migration default. A delivery repeats the 0.8-second chime with a 0.3-second pause. One blocking worker sleeps on a bounded channel, opens the current default output for each chime, keeps the stream alive through playback, then releases it. At most one signal waits behind the active one; bursts coalesce. Dismiss, Snooze, Restart, disabling sound and changing duration invalidate the active request and stop it promptly. Playback completion has a three-second bound after opening the device, allowing a later request to retry after stream failure. Device initialization itself is controlled by the OS/backend and is not covered by that bound.

## Consequences and Validation

Windows and macOS use their native audio backends through CPAL. Linux requires ALSA development headers (`libasound2-dev` on Debian/Ubuntu) to build and the ALSA runtime library plus a working output device to play; CI and release workflows install the headers. No new external player is required.

Pure tests cover bounded, finite, non-silent samples, smooth boundaries and queue saturation/disconnection. An ignored hardware smoke test auditions the production playback path. Cross-platform compilation and real listening remain separate acceptance checks; successful playback API calls do not establish audibility.

Size comparison uses the same source baseline with the preceding Windows build fix, Rust toolchain, release profile and Windows packaging command. On Windows x64 with rustc 1.98.0, static CRT, `opt-level = "z"`, thin LTO and stripped symbols:

| Artifact | Before (bytes) | After (bytes) | Increase |
| --- | ---: | ---: | ---: |
| Release executable | 14,949,376 | 15,063,040 | 113,664 (0.76%) |
| ZIP with executable and Carlito license | 6,661,536 | 6,709,887 | 48,351 (0.73%) |

Windows validation: formatting, Clippy with warnings denied, 80 passing automated workspace tests plus one ignored hardware smoke test, debug/release builds and the explicit hardware playback test passed. The hardware test completed in 0.85 seconds without an audio API error; audibility was independently confirmed on Windows. The debug application was launched. `cargo tree -i rodio --edges features` confirmed only the `playback` feature. The signal allocates 153,600 bytes of sample data while playing; the worker waits on the channel without an open audio stream when idle. Whole-process memory/CPU differences were not benchmarked.

macOS and Linux compilation, actual listening, Finder launch, sleep/resume and headphone/output switching still require platform verification. Windows size results do not predict exact deltas on those platforms.
