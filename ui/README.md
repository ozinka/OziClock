# Slint UI

This directory owns shared Slint components, bundled fonts, and future design tokens. Active application windows and feature-specific components currently live in `apps/oziclock-desktop/ui/` and are compiled by the desktop crate.

Move a component here when it is genuinely shared; do not duplicate application rules in either UI directory. Rust domain and application crates own behavior, while Slint owns declarative composition, rendering, and user-gesture callbacks.
