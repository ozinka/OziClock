# Minimal Windows Development Environment

This guide describes the smallest supported setup required to build OziClock with Codex. No editor or IDE is required: Codex works with the repository and command-line tools directly.

## Required Once

### 1. Microsoft Visual Studio Community

Install the current **Visual Studio Community** edition. It is free for individual developers. The IDE will not be used for day-to-day work; it provides the Microsoft C++ linker and Windows SDK required by Rust's supported Windows MSVC toolchain.

In the installer, select only the **Desktop development with C++** workload. Ensure it includes:

- the latest **MSVC C++ x64/x86 build tools**;
- a **Windows 10 or Windows 11 SDK**.

Do not install Professional, Enterprise, or extra workloads. Do not manually add Visual Studio folders to `PATH`; Rust and Cargo discover the MSVC toolchain automatically.

### 2. Rust Stable Toolchain

Download and run the x64 `rustup-init.exe` installer from <https://rust-lang.org/tools/install/>. Choose the default stable **`x86_64-pc-windows-msvc`** installation. It installs `rustc`, `cargo`, and `rustup` for the current user.

`rustup` normally adds this directory to the user `PATH`:

```text
%USERPROFILE%\.cargo\bin
```

Close and reopen PowerShell and Codex after installation. If `cargo` is still not found, add that exact directory to the user `PATH`, then reopen the terminal. Do not add arbitrary Rust, MSVC, or Windows SDK directories manually.

### 3. Rust Quality Tools

From a new PowerShell session, run:

```powershell
rustup component add rustfmt clippy
rustc --version
cargo --version
```

These tools format Rust code and detect common mistakes. They are required by the repository workflow.

## Verify OziClock

Open PowerShell in the repository root and run:

```powershell
cargo check --workspace
cargo test --workspace
cargo run -p oziclock-desktop
```

The current scaffold prints `OziClock`. Later, Cargo will download Slint and all Rust libraries declared by the project; do not install Slint, a GUI SDK, CMake, Python, Node.js, or a Rust IDE in advance.

## Optional Tools

- **JetBrains RustRover or VS Code:** useful only if you want to inspect or edit code yourself. Neither is required for Codex.
- **.NET 9 SDK:** required only to build `legacy/dotnet-wpf/`; it is not needed for the Rust rewrite.
- **Git:** already available in the current workspace. Keep it installed for source control.

## Later Platform Setup

Windows is the first development target. macOS and Linux packaging will require their native build environments on those operating systems or CI runners; they are not needed now and should not be emulated through extra local tooling.

