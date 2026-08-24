# Visual Testing

## Current Process

The ruler/lens UI is rendered by Slint and must be visually reviewed on a desktop build. Before a visual release, capture reviewed reference images for these cases:

- Lens at the top, center, and bottom positions.
- A non-main Column slider position.
- A time zone with a 30- or 45-minute offset.
- Standard and compact clock modes at 100%, 125%, 150%, and 200% display scale.

Compare each capture against the current approved reference in `legacy/dotnet-wpf/Ozi.Clock/Assets/` and the latest product screenshots. Check borders, one-pixel joins, label baselines, lens clipping, and attached-window alignment.

## Automation Boundary

The project does not yet have a deterministic headless Slint screenshot renderer. Do not commit unreviewed images as golden baselines. The CI workflow runs formatting, linting, tests, and release builds on Windows, Linux, and macOS; visual golden-image automation will be added when a stable renderer harness is available.
