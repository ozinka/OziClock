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
const SOUND_PRESET_COUNT: u8 = 6;

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
        self.selected_sound.store(
            u64::from(sound_id.min(SOUND_PRESET_COUNT - 1)),
            Ordering::Relaxed,
        );
        self.stop();
    }

    pub(super) fn preview(&self, sound_id: u8) {
        self.stop();
        if let Some(sender) = &self.sender {
            let request = SoundRequest {
                generation: self.generation.load(Ordering::Relaxed),
                until: Instant::now() + Duration::from_secs(2),
                sound_id: sound_id.min(SOUND_PRESET_COUNT - 1),
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
    (0..count)
        .map(|index| {
            let time = index as f32 / SAMPLE_RATE as f32;
            match sound_id {
                1 => woodblock_sample(time),
                2 => marimba_sample(time),
                3 => double_bell_sample(time),
                4 => digital_chirp_sample(time),
                5 => original_chime_sample(time),
                _ => bell_sample(time, 880.0),
            }
        })
        .collect()
}

fn original_chime_sample(time: f32) -> f32 {
    let strike = attack(time, 0.012);
    let body = (-4.0 * time).exp() * final_fade(time, 0.34);
    let fundamental = (TAU * 880.0 * time).sin();
    let overtone = (TAU * 1_320.0 * time).sin();
    0.25 * strike * body * (fundamental + 0.35 * overtone)
}

fn bell_sample(time: f32, frequency: f32) -> f32 {
    let strike = attack(time, 0.006);
    let body = (-3.8 * time).exp() * final_fade(time, 0.34);
    let fundamental = (TAU * frequency * time).sin() * (-3.0 * time).exp();
    let inharmonic_partial = (TAU * frequency * 2.76 * time).sin() * 0.42 * (-7.0 * time).exp();
    let high_partial = (TAU * frequency * 5.4 * time).sin() * 0.16 * (-12.0 * time).exp();
    0.20 * strike * body * (fundamental + inharmonic_partial + high_partial)
}

fn woodblock_sample(time: f32) -> f32 {
    let attack = attack(time, 0.0015);
    let body = (-32.0 * time).exp();
    let low_resonance = (TAU * 920.0 * time).sin();
    let hard_resonance = (TAU * 2_380.0 * time).sin() * 0.32;
    0.25 * attack * body * (low_resonance + hard_resonance)
}

fn marimba_sample(time: f32) -> f32 {
    let attack = attack(time, 0.004);
    let decay = (-8.0 * time).exp() * final_fade(time, 0.32);
    let phase = TAU * (659.0 * time + 95.0 * (1.0 - (-18.0 * time).exp()) / 18.0);
    let fundamental = phase.sin();
    let second_partial = (phase * 2.01).sin() * 0.52 * (-12.0 * time).exp();
    let third_partial = (phase * 3.9).sin() * 0.18 * (-20.0 * time).exp();
    0.20 * attack * decay * (fundamental + second_partial + third_partial)
}

fn double_bell_sample(time: f32) -> f32 {
    let first = bell_voice(time, 784.0);
    let second = bell_voice(time - 0.34, 1_046.5);
    0.12 * (first + second) * final_fade(time, 0.34)
}

fn bell_voice(time: f32, frequency: f32) -> f32 {
    if time < 0.0 {
        return 0.0;
    }
    let fundamental = (TAU * frequency * time).sin() * (-4.5 * time).exp();
    let overtone = (TAU * frequency * 2.71 * time).sin() * 0.38 * (-9.0 * time).exp();
    attack(time, 0.004) * (fundamental + overtone)
}

fn digital_chirp_sample(time: f32) -> f32 {
    let mut sample = 0.0;
    for (start, base_hz) in [(0.0, 560.0), (0.29, 820.0)] {
        let local_time = time - start;
        let duration = 0.19;
        if (0.0..duration).contains(&local_time) {
            let progress = local_time / duration;
            let frequency_sweep =
                base_hz * local_time + 0.5 * base_hz * 1.5 * local_time * progress;
            let envelope =
                attack(local_time, 0.009) * ((duration - local_time) / 0.035).clamp(0.0, 1.0);
            sample += (TAU * frequency_sweep).sin() * envelope;
        }
    }
    0.30 * sample
}

fn attack(time: f32, seconds: f32) -> f32 {
    (time / seconds).clamp(0.0, 1.0)
}

fn final_fade(time: f32, seconds: f32) -> f32 {
    ((SIGNAL_SECONDS - 1.0 / SAMPLE_RATE as f32 - time) / seconds).clamp(0.0, 1.0)
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
                .all(|pair| (pair[1] - pair[0]).abs() < 0.25)
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
        for sound_id in 1..SOUND_PRESET_COUNT {
            let samples = signal_samples(sound_id);
            assert_ne!(classic, samples);
            assert!(samples.iter().all(|sample| sample.abs() <= 0.34));
            assert!(
                samples
                    .windows(2)
                    .all(|pair| (pair[1] - pair[0]).abs() < 0.25)
            );
        }
        let samples = &classic[classic.len() - (SAMPLE_RATE as usize / 3)..];
        let start = samples.first().unwrap().abs();
        let end = samples.last().unwrap().abs();
        assert!(start > end);
        assert_eq!(end, 0.0);
    }

    #[test]
    fn presets_use_distinct_percussive_and_tonal_envelopes() {
        let woodblock = signal_samples(1);
        let marimba = signal_samples(2);
        let double_bell = signal_samples(3);
        let chirp = signal_samples(4);
        let energy = |samples: &[f32], start: usize, end: usize| {
            samples[start..end]
                .iter()
                .map(|sample| sample * sample)
                .sum::<f32>()
        };
        let late_start = SAMPLE_RATE as usize / 4;
        assert!(
            energy(&woodblock, late_start, woodblock.len())
                < energy(&marimba, late_start, marimba.len())
        );
        assert!(energy(&double_bell, late_start, double_bell.len()) > 0.0);
        assert_eq!(energy(&chirp, SAMPLE_RATE as usize / 2, chirp.len()), 0.0);
    }
}
