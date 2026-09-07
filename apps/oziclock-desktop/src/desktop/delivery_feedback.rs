use super::alert_sound::AlertSound;

pub(super) fn deliver(title: &str, body: &str, sound_seconds: u8, sound: &AlertSound) {
    if sound_seconds > 0 {
        sound.play_for_seconds(sound_seconds);
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
