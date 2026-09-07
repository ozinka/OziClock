pub fn deliver(title: &str, body: &str, play_sound: bool) {
    if let Err(error) = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .appname("OziClock")
        .show()
    {
        eprintln!("Native notification unavailable: {error}");
    }
    if play_sound {
        play_sound_once();
    }
}

#[cfg(target_os = "macos")]
fn play_sound_once() {
    objc2_app_kit::NSBeep();
}

#[cfg(not(target_os = "macos"))]
fn play_sound_once() {
    print!("\u{7}");
}
