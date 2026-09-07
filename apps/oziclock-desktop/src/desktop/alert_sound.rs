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

const SAMPLE_RATE: u32 = 48_000;
const SIGNAL_SECONDS: f32 = 0.8;

/// One application-owned worker; the bounded queue prevents alert bursts from
/// creating unbounded threads or a long backlog of sounds.
#[derive(Clone)]
pub(super) struct AlertSound {
    sender: Option<SyncSender<SoundRequest>>,
    generation: Arc<AtomicU64>,
}

impl AlertSound {
    pub(super) fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        let generation = Arc::new(AtomicU64::new(0));
        let worker_generation = generation.clone();
        match thread::Builder::new()
            .name("oziclock-audio".into())
            .spawn(move || {
                while let Ok(request) = receiver.recv() {
                    if let Err(error) = play_for(request, &worker_generation) {
                        eprintln!("Alert sound unavailable: {error}");
                    }
                }
            }) {
            Ok(_) => Self {
                sender: Some(sender),
                generation,
            },
            Err(error) => {
                eprintln!("Could not start alert audio worker: {error}");
                Self {
                    sender: None,
                    generation,
                }
            }
        }
    }

    pub(super) fn play_for_seconds(&self, seconds: u8) {
        if seconds == 0 {
            self.stop();
            return;
        }
        if let Some(sender) = &self.sender {
            let generation = self.generation.load(Ordering::Relaxed);
            let request = SoundRequest {
                generation,
                until: Instant::now() + Duration::from_secs(u64::from(seconds)),
            };
            match sender.try_send(request) {
                Ok(()) => {}
                Err(TrySendError::Disconnected(_)) => {
                    eprintln!("Alert audio worker disconnected");
                }
                Err(TrySendError::Full(_)) => {}
            }
        }
    }

    pub(super) fn stop(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Clone, Copy)]
struct SoundRequest {
    generation: u64,
    until: Instant,
}

fn signal_samples() -> Vec<f32> {
    let count = (SAMPLE_RATE as f32 * SIGNAL_SECONDS) as usize;
    (0..count)
        .map(|index| {
            let time = index as f32 / SAMPLE_RATE as f32;
            let attack = (time / 0.012).min(1.0);
            let release = ((count - 1 - index) as f32 / SAMPLE_RATE as f32 / 0.08).min(1.0);
            let envelope = attack * release * (-4.0 * time).exp();
            let fundamental = (TAU * 880.0 * time).sin();
            let overtone = (TAU * 1320.0 * time).sin();
            0.25 * envelope * (fundamental + 0.35 * overtone)
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
    let mut device = DeviceSinkBuilder::open_default_sink().map_err(|error| error.to_string())?;
    device.log_on_drop(false);
    let player = Player::connect_new(device.mixer());
    player.append(SamplesBuffer::new(
        nz!(1),
        std::num::NonZeroU32::new(SAMPLE_RATE).expect("sample rate is nonzero"),
        signal_samples(),
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
        let samples = signal_samples();
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
            },
            &generation,
        )
        .expect("alert playback should complete on the default audio device");
    }
}
