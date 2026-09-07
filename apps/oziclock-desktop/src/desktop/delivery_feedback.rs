pub fn deliver(title: &str, body: &str, play_sound: bool) {
    if play_sound {
        play_sound_once();
    }
    if let Err(error) = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .appname("OziClock")
        .show()
    {
        eprintln!("Native notification unavailable: {error}");
    }
}

#[cfg(target_os = "macos")]
fn play_sound_once() {
    if let Err(error) = std::process::Command::new("/usr/bin/afplay")
        .arg("/System/Library/Sounds/Glass.aiff")
        .spawn()
    {
        eprintln!("Could not start alert sound: {error}");
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn play_sound_once() {
    if let Err(error) = std::process::Command::new("canberra-gtk-play")
        .args(["--id=message-new-instant", "--description=OziClock alert"])
        .spawn()
    {
        eprintln!("Could not start alert sound: {error}");
    }
}

#[cfg(target_os = "windows")]
fn play_sound_once() {
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::MessageBeep(0);
    }
}
