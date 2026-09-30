use std::{
    f32::consts::TAU,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};

use rodio::{DeviceSinkBuilder, Player, buffer::SamplesBuffer, nz};

use super::diagnostics;

const SAMPLE_RATE: u32 = 48_000;
const SIGNAL_SECONDS: f32 = 0.8;
const DEFAULT_SOUND_ID: u8 = 0;

/// One application-owned worker; the bounded queue prevents alert bursts from
/// creating unbounded threads or a long backlog of sounds.
#[derive(Clone)]
pub(super) struct AlertSound {
    sender: Option<SyncSender<SoundRequest>>,
    generation: Arc<AtomicU64>,
    selected_sound: Arc<AtomicU64>,
}

impl AlertSound {
    pub(super) fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel::<SoundRequest>(1);
        let generation = Arc::new(AtomicU64::new(0));
        let selected_sound = Arc::new(AtomicU64::new(u64::from(DEFAULT_SOUND_ID)));
        let worker_generation = generation.clone();
        match thread::Builder::new()
            .name("oziclock-audio".into())
            .spawn(move || {
                while let Ok(request) = receiver.recv() {
                    diagnostics::record(&format!(
                        "audio-request-received duration-seconds={}",
                        request
                            .until
                            .saturating_duration_since(Instant::now())
                            .as_secs()
                    ));
                    if let Err(error) = play_for(request, &worker_generation) {
                        diagnostics::record(&format!("audio-request-failed error={error}"));
                        eprintln!("Alert sound unavailable: {error}");
                    } else {
                        diagnostics::record("audio-request-completed");
                    }
                }
            }) {
            Ok(_) => Self {
                sender: Some(sender),
                generation,
                selected_sound,
            },
            Err(error) => {
                eprintln!("Could not start alert audio worker: {error}");
                Self {
                    sender: None,
                    generation,
                    selected_sound,
                }
            }
        }
    }

    pub(super) fn play_for_seconds(&self, seconds: u8) {
        if seconds == 0 {
            self.stop();
            diagnostics::record("audio-request-cancelled sound-disabled");
            return;
        }
        if let Some(sender) = &self.sender {
            let generation = self.generation.load(Ordering::Relaxed);
            let request = SoundRequest {
                generation,
                until: Instant::now() + Duration::from_secs(u64::from(seconds)),
                sound_id: self.selected_sound.load(Ordering::Relaxed) as u8,
            };
            match sender.try_send(request) {
                Ok(()) => {}
                Err(TrySendError::Disconnected(_)) => {
                    diagnostics::record("audio-request-dropped worker-disconnected");
                    eprintln!("Alert audio worker disconnected");
                }
                Err(TrySendError::Full(_)) => diagnostics::record("audio-request-coalesced"),
            }
        }
    }

    pub(super) fn stop(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn set_sound(&self, sound_id: u8) {
        self.selected_sound
            .store(u64::from(sound_id.min(3)), Ordering::Relaxed);
        self.stop();
    }

    pub(super) fn preview(&self, sound_id: u8) {
        self.stop();
        if let Some(sender) = &self.sender {
            let request = SoundRequest {
                generation: self.generation.load(Ordering::Relaxed),
                until: Instant::now() + Duration::from_secs(2),
                sound_id: sound_id.min(3),
            };
            let _ = sender.try_send(request);
        }
    }
}

#[derive(Clone, Copy)]
struct SoundRequest {
    generation: u64,
    until: Instant,
    sound_id: u8,
}

fn signal_samples(sound_id: u8) -> Vec<f32> {
    let count = (SAMPLE_RATE as f32 * SIGNAL_SECONDS) as usize;
    let (fundamental_hz, overtone_hz, overtone_gain, release_seconds) = match sound_id {
        1 => (1046.5, 1568.0, 0.62, 0.30),
        2 => (659.3, 987.8, 0.28, 0.48),
        3 => (880.0, 1320.0, 0.58, 0.34),
        _ => (880.0, 1320.0, 0.35, 0.34),
    };
    (0..count)
        .map(|index| {
            let time = index as f32 / SAMPLE_RATE as f32;
            let attack = (time / 0.012).min(1.0);
            let release =
                ((count - 1 - index) as f32 / SAMPLE_RATE as f32 / release_seconds).min(1.0);
            let envelope = attack * release * (-3.2 * time).exp();
            let double_chime = if sound_id == 3 {
                0.35 + 0.65 * (TAU * 1.25 * time).cos().powi(2)
            } else {
                1.0
            };
            let fundamental = (TAU * fundamental_hz * time).sin();
            let overtone = (TAU * overtone_hz * time).sin();
            0.25 * envelope * double_chime * (fundamental + overtone_gain * overtone)
        })
        .collect()
}

fn play_for(request: SoundRequest, generation: &AtomicU64) -> Result<(), String> {
    while Instant::now() < request.until && generation.load(Ordering::Relaxed) == request.generation
    {
        play_signal(request, generation)?;
        if Instant::now() + Duration::from_millis(300) >= request.until {
            break;
        }
        let pause_until = Instant::now() + Duration::from_millis(300);
        while Instant::now() < pause_until {
            if generation.load(Ordering::Relaxed) != request.generation {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    Ok(())
}

fn play_signal(request: SoundRequest, generation: &AtomicU64) -> Result<(), String> {
    // Reopen for each signal to pick up output-device changes and release the
    // audio device while idle. Keep both handles alive until playback completes.
    diagnostics::record("audio-device-open-start");
    let mut device = DeviceSinkBuilder::open_default_sink().map_err(|error| error.to_string())?;
    diagnostics::record("audio-device-opened");
    device.log_on_drop(false);
    let player = Player::connect_new(device.mixer());
    player.append(SamplesBuffer::new(
        nz!(1),
        std::num::NonZeroU32::new(SAMPLE_RATE).expect("sample rate is nonzero"),
        signal_samples(request.sound_id),
    ));
    let deadline = Instant::now()
        + Duration::from_secs(3).min(request.until.saturating_duration_since(Instant::now()));
    while !player.empty() {
        if generation.load(Ordering::Relaxed) != request.generation {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("audio device did not finish the signal within three seconds".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_is_finite_audible_and_has_smooth_silent_edges() {
        let samples = signal_samples(0);
        assert_eq!(samples.len(), 38_400);
        assert_eq!(samples[0], 0.0);
        assert_eq!(*samples.last().unwrap(), 0.0);
        assert!(
            samples
                .iter()
                .all(|sample| sample.is_finite() && sample.abs() <= 0.34)
        );
        let energy: f32 = samples.iter().map(|sample| sample * sample).sum();
        assert!(energy / samples.len() as f32 > 0.001);
        assert!(
            samples
                .windows(2)
                .all(|pair| (pair[1] - pair[0]).abs() < 0.06)
        );
    }

    #[test]
    fn alert_burst_is_bounded_and_disconnection_is_nonfatal() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let sound = AlertSound {
            sender: Some(sender),
            generation: Arc::new(AtomicU64::new(0)),
            selected_sound: Arc::new(AtomicU64::new(0)),
        };
        for _ in 0..100 {
            sound.play_for_seconds(20);
        }
        assert_eq!(receiver.try_iter().count(), 1);
        drop(receiver);
        sound.play_for_seconds(20);
    }

    #[test]
    fn stop_invalidates_active_playback_generation() {
        let sound = AlertSound {
            sender: None,
            generation: Arc::new(AtomicU64::new(7)),
            selected_sound: Arc::new(AtomicU64::new(0)),
        };
        sound.stop();
        assert_eq!(sound.generation.load(Ordering::Relaxed), 8);
    }

    #[test]
    #[ignore = "requires a real audio output; run manually to audition the alert"]
    fn play_alert_on_default_device() {
        let generation = AtomicU64::new(0);
        play_for(
            SoundRequest {
                generation: 0,
                until: Instant::now() + Duration::from_secs(1),
                sound_id: 0,
            },
            &generation,
        )
        .expect("alert playback should complete on the default audio device");
    }

    #[test]
    fn sound_presets_are_distinct_and_fade_over_at_least_three_tenths_of_a_second() {
        let classic = signal_samples(0);
        for sound_id in 1..=3 {
            assert_ne!(classic, signal_samples(sound_id));
        }
        let samples = &classic[classic.len() - (SAMPLE_RATE as usize / 3)..];
        let start = samples.first().unwrap().abs();
        let end = samples.last().unwrap().abs();
        assert!(start > end);
        assert_eq!(end, 0.0);
    }
}
