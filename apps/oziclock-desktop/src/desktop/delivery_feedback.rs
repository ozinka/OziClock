use super::{alert_sound::AlertSound, diagnostics};

pub(super) fn deliver(title: &str, body: &str, sound_seconds: u8, sound: &AlertSound) {
    request_sound(sound_seconds, sound);
    if let Err(error) = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .appname("OziClock")
        .show()
    {
        diagnostics::record(&format!("native-notification-failed error={error}"));
        eprintln!("Native notification unavailable: {error}");
    } else {
        diagnostics::record("native-notification-requested");
    }
}

pub(super) fn deliver_in_window(sound_seconds: u8, sound: &AlertSound) {
    request_sound(sound_seconds, sound);
    diagnostics::record("delivery-presented-in-window");
}

fn request_sound(sound_seconds: u8, sound: &AlertSound) {
    diagnostics::record(&format!("delivery-start sound-seconds={sound_seconds}"));
    if sound_seconds > 0 {
        sound.play_for_seconds(sound_seconds);
    }
}
