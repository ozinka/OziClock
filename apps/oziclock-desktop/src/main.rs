#![windows_subsystem = "windows"]

mod desktop;

fn main() -> Result<(), slint::PlatformError> {
    desktop::run()
}
