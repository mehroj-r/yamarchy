use anyhow::{Context, Result};
use bytes::Bytes;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct FadeIn {
    duration: Duration,
    elapsed: Duration,
    last_tick: Instant,
}

struct FadingSink {
    sink: Sink,
    start_vol_ratio: f32,
    duration: Duration,
    elapsed: Duration,
    last_tick: Instant,
}

pub struct AudioPlayerEngine {
    stream_handle: OutputStreamHandle,
    active_sink: Option<Sink>,
    active_fade: Option<FadeIn>,
    fading_out_sinks: Vec<FadingSink>,
    volume: f32,
    is_playing: Arc<AtomicBool>,
}

// Safety: OutputStream is held in a leaked static container for daemon lifetime,
// and OutputStreamHandle + Sink are Send.
unsafe impl Send for AudioPlayerEngine {}

impl AudioPlayerEngine {
    pub fn try_new() -> Result<Self> {
        let (stream, stream_handle) =
            OutputStream::try_default().context("Failed to initialize audio output stream")?;
        // Keep the stream alive for the entire process lifetime
        Box::leak(Box::new(stream));
        let is_playing = Arc::new(AtomicBool::new(false));

        Ok(Self {
            stream_handle,
            active_sink: None,
            active_fade: None,
            fading_out_sinks: Vec::new(),
            volume: 0.8,
            is_playing,
        })
    }

    pub fn play_bytes(&mut self, audio_bytes: Bytes) -> Result<()> {
        // Stop all fading sinks immediately
        for fading in self.fading_out_sinks.drain(..) {
            fading.sink.stop();
        }

        if let Some(sink) = self.active_sink.take() {
            sink.stop();
        }
        self.active_fade = None;

        let new_sink =
            Sink::try_new(&self.stream_handle).context("Failed to recreate audio sink")?;
        new_sink.set_volume(self.volume);

        let cursor = Cursor::new(audio_bytes);
        let source = Decoder::new(cursor).context("Failed to decode audio bytes with Rodio")?;
        new_sink.append(source);
        new_sink.play();

        self.active_sink = Some(new_sink);
        self.is_playing.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn crossfade_to_bytes(&mut self, audio_bytes: Bytes, duration: Duration) -> Result<()> {
        if duration.is_zero() {
            return self.play_bytes(audio_bytes);
        }

        let now = Instant::now();

        // If there is an active sink, move it to fading_out_sinks
        if let Some(old_sink) = self.active_sink.take() {
            let current_ratio = if let Some(ref fade) = self.active_fade {
                (fade.elapsed.as_secs_f32() / fade.duration.as_secs_f32()).clamp(0.0, 1.0)
            } else {
                1.0
            };

            self.fading_out_sinks.push(FadingSink {
                sink: old_sink,
                start_vol_ratio: current_ratio,
                duration,
                elapsed: Duration::ZERO,
                last_tick: now,
            });
        }

        let new_sink =
            Sink::try_new(&self.stream_handle).context("Failed to create crossfade audio sink")?;
        new_sink.set_volume(0.0);

        let cursor = Cursor::new(audio_bytes);
        let source = Decoder::new(cursor).context("Failed to decode audio bytes with Rodio")?;
        new_sink.append(source);
        new_sink.play();

        self.active_sink = Some(new_sink);
        self.active_fade = Some(FadeIn {
            duration,
            elapsed: Duration::ZERO,
            last_tick: now,
        });
        self.is_playing.store(true, Ordering::SeqCst);

        Ok(())
    }

    pub fn tick(&mut self) {
        if !self.is_playing.load(Ordering::SeqCst) {
            return;
        }

        let now = Instant::now();

        // 1. Advance active sink fade-in
        if let Some(ref mut fade) = self.active_fade {
            let dt = now.saturating_duration_since(fade.last_tick);
            fade.last_tick = now;
            fade.elapsed += dt;

            if fade.elapsed >= fade.duration {
                if let Some(ref sink) = self.active_sink {
                    sink.set_volume(self.volume);
                }
                self.active_fade = None;
            } else {
                let ratio =
                    (fade.elapsed.as_secs_f32() / fade.duration.as_secs_f32()).clamp(0.0, 1.0);
                if let Some(ref sink) = self.active_sink {
                    sink.set_volume(ratio * self.volume);
                }
            }
        }

        // 2. Advance outgoing fading sinks
        let vol = self.volume;
        self.fading_out_sinks.retain_mut(|fading| {
            let dt = now.saturating_duration_since(fading.last_tick);
            fading.last_tick = now;
            fading.elapsed += dt;

            if fading.elapsed >= fading.duration || fading.sink.empty() {
                fading.sink.stop();
                false
            } else {
                let progress =
                    (fading.elapsed.as_secs_f32() / fading.duration.as_secs_f32()).clamp(0.0, 1.0);
                let ratio = (1.0 - progress) * fading.start_vol_ratio;
                fading.sink.set_volume((ratio * vol).clamp(0.0, 1.0));
                true
            }
        });
    }

    pub fn pause(&mut self) {
        if let Some(ref sink) = self.active_sink {
            sink.pause();
        }
        for fading in &self.fading_out_sinks {
            fading.sink.pause();
        }
        self.is_playing.store(false, Ordering::SeqCst);
    }

    pub fn resume(&mut self) {
        let now = Instant::now();
        if let Some(ref sink) = self.active_sink {
            sink.play();
        }
        if let Some(ref mut fade) = self.active_fade {
            fade.last_tick = now;
        }
        for fading in &mut self.fading_out_sinks {
            fading.sink.play();
            fading.last_tick = now;
        }
        self.is_playing.store(true, Ordering::SeqCst);
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.active_sink.take() {
            sink.stop();
        }
        self.active_fade = None;
        for fading in self.fading_out_sinks.drain(..) {
            fading.sink.stop();
        }
        self.is_playing.store(false, Ordering::SeqCst);
    }

    pub fn is_empty(&self) -> bool {
        self.active_sink.as_ref().map(|s| s.empty()).unwrap_or(true)
    }

    pub fn is_paused(&self) -> bool {
        self.active_sink
            .as_ref()
            .map(|s| s.is_paused())
            .unwrap_or(true)
    }

    pub fn get_position(&self) -> Duration {
        self.active_sink
            .as_ref()
            .map(|s| s.get_pos())
            .unwrap_or(Duration::ZERO)
    }

    pub fn set_volume(&mut self, vol: f32) {
        let clamped = vol.clamp(0.0, 1.0);
        self.volume = clamped;

        // Apply updated volume proportionally
        if let Some(ref fade) = self.active_fade {
            let ratio = (fade.elapsed.as_secs_f32() / fade.duration.as_secs_f32()).clamp(0.0, 1.0);
            if let Some(ref sink) = self.active_sink {
                sink.set_volume(ratio * self.volume);
            }
        } else if let Some(ref sink) = self.active_sink {
            sink.set_volume(self.volume);
        }

        for fading in &self.fading_out_sinks {
            let progress =
                (fading.elapsed.as_secs_f32() / fading.duration.as_secs_f32()).clamp(0.0, 1.0);
            let ratio = (1.0 - progress) * fading.start_vol_ratio;
            fading
                .sink
                .set_volume((ratio * self.volume).clamp(0.0, 1.0));
        }
    }

    pub fn get_volume(&self) -> f32 {
        self.volume
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_clamp() {
        if let Ok(mut engine) = AudioPlayerEngine::try_new() {
            engine.set_volume(1.5);
            assert_eq!(engine.get_volume(), 1.0);

            engine.set_volume(-0.5);
            assert_eq!(engine.get_volume(), 0.0);

            engine.set_volume(0.65);
            assert_eq!(engine.get_volume(), 0.65);
        }
    }
}
