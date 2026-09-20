#![windows_subsystem = "windows"]

mod desktop;

fn main() -> Result<(), slint::PlatformError> {
    desktop::install_diagnostic_panic_hook();
    desktop::run()
}
