# ADR 0001: Cross-Platform UI Stack

- Status: Superseded by ADR 0002
- Date: 2026-08-23

## Context

The WPF application must be rewritten in another language and run on Windows, macOS, and Linux. Its distinguishing requirement is a compact, precisely positioned custom UI rather than native-looking form controls. The application also needs frameless and always-on-top windows, context menus, settings persistence, time-zone data, and distributable desktop packages.

## Decision

Use **Flutter with Dart** for the rewrite, subject to a one-screen prototype on all three desktop platforms.

Flutter renders through its own graphics pipeline, making tile dimensions, font placement, gradients, rulers, and snapshot testing more consistent across operating systems. Dart keeps UI and application code in one language, supports fast iteration, and has mature desktop targets. Platform-specific window behavior should sit behind a small adapter so plugins can be replaced or supplemented with native code where necessary.

The prototype must validate:

- frameless, draggable, transparent, always-on-top behavior;
- correct DPI scaling and font metrics at 100%, 125%, and 200%;
- a horizontal strip of at least eight tiles without seams;
- context menus and persistence;
- IANA time zones, daylight-saving transitions, and half-hour offsets;
- acceptable idle CPU and memory usage.

## Alternatives Considered

- **Avalonia/C#:** lowest migration effort and a strong desktop fit, but does not satisfy the goal of changing language and keeps the design close to the current .NET architecture.
- **Rust + Slint:** potentially smaller and resource-efficient with good custom rendering, but brings higher implementation cost and a less mature desktop ecosystem. Reconsider if prototype resource usage is unacceptable.
- **Tauri + TypeScript:** small native shell and broad web ecosystem, but system webviews can vary in typography and rendering, increasing risk for pixel-sensitive layouts.
- **Qt/QML + C++:** capable and mature, but has greater language complexity and licensing/deployment considerations for this small application.

## Consequences

The UI will be rebuilt rather than mechanically translated from XAML. Exact appearance should be expressed as reusable design tokens and guarded by golden-image tests. Flutter adds runtime size compared with a minimal native executable; package size is secondary to rendering consistency, while idle resource usage remains an explicit acceptance criterion.

## References

- [Flutter desktop support](https://docs.flutter.dev/platform-integration/desktop)
- [Flutter architectural overview](https://docs.flutter.dev/resources/architectural-overview)
- [Flutter platform-specific code](https://docs.flutter.dev/platform-integration/platform-channels)
- [Tauri architecture](https://tauri.app/concept/architecture/)
- [Tauri webview versions](https://tauri.app/reference/webview-versions/)
