use anyhow::Result;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::broadcast;
use yamarchy_core::client::YandexMusicClient;
use yamarchy_core::models::{
    AudioSettings, PlaybackStatus, PlayerState, RotorBatchResponse, Track, TrackSummary,
    VibeSettings,
};
use yamarchy_core::rotor::RotorFeedbackEvent;
use yamarchy_player::engine::AudioPlayerEngine;
use yamarchy_player::stream::AudioStreamFetcher;

use crate::config::Config;
use crate::socket::SocketCommand;

pub struct PlayerService {
    config: Config,
    client: YandexMusicClient,
    engine: AudioPlayerEngine,
    fetcher: AudioStreamFetcher,
    state: PlayerState,
    current_track_full: Option<Track>,
    current_batch_id: Option<String>,
    queue: VecDeque<Track>,
    history: Vec<Track>,
    prefetched_next: Option<(Track, bytes::Bytes)>,
    is_prefetching: bool,
    user_uid: Option<u64>,
    state_tx: broadcast::Sender<PlayerState>,
}

impl PlayerService {
    pub async fn new(config: Config, state_tx: broadcast::Sender<PlayerState>) -> Result<Self> {
        let client = YandexMusicClient::new(config.auth.oauth_token.clone());
        let engine = AudioPlayerEngine::try_new()?;
        let fetcher = AudioStreamFetcher::new();

        let initial_vibe = VibeSettings {
            diversity: config.vibe.diversity.clone(),
            mood_energy: config.vibe.mood_energy.clone(),
            language: config.vibe.language.clone(),
        };

        let initial_audio = AudioSettings {
            crossfade_enabled: config.audio.crossfade_enabled,
            crossfade_duration: config.audio.crossfade_duration.clamp(1, 5),
            crossfade_on_manual_skip: config.audio.crossfade_on_manual_skip,
        };

        let initial_state = PlayerState {
            status: PlaybackStatus::Stopped,
            current_track: None,
            position_ms: 0,
            duration_ms: 0,
            volume: config.audio.volume,
            station: Some(config.vibe.station.clone()),
            vibe_settings: initial_vibe,
            audio_settings: initial_audio,
            queue: Vec::new(),
            liked: false,
            shuffle: false,
            repeat: false,
        };

        let mut service = Self {
            config,
            client,
            engine,
            fetcher,
            state: initial_state,
            current_track_full: None,
            current_batch_id: None,
            queue: VecDeque::new(),
            history: Vec::new(),
            prefetched_next: None,
            is_prefetching: false,
            user_uid: None,
            state_tx,
        };

        // Try getting user account info
        if !service.config.auth.oauth_token.is_empty() {
            if let Ok(account) = service.client.get_account_status().await {
                if let Some(acc) = account.account {
                    service.user_uid = acc.uid;
                }
            }
        }

        Ok(service)
    }

    pub fn state(&self) -> PlayerState {
        let mut state = self.state.clone();
        if state.status == PlaybackStatus::Playing {
            state.position_ms = self.engine.get_position().as_millis() as u64;
        }
        state
    }

    pub async fn handle_command(&mut self, cmd: SocketCommand) -> Result<PlayerState> {
        match cmd {
            SocketCommand::Play => {
                if self.state.status == PlaybackStatus::Paused {
                    self.engine.resume();
                    self.state.status = PlaybackStatus::Playing;
                    self.broadcast_state();
                } else if self.state.status == PlaybackStatus::Stopped {
                    self.play_next_internal(false).await?;
                }
            }
            SocketCommand::Pause => {
                if self.state.status == PlaybackStatus::Playing {
                    self.engine.pause();
                    self.state.status = PlaybackStatus::Paused;
                    self.broadcast_state();
                }
            }
            SocketCommand::Toggle => {
                if self.state.status == PlaybackStatus::Playing {
                    self.engine.pause();
                    self.state.status = PlaybackStatus::Paused;
                    self.broadcast_state();
                } else if self.state.status == PlaybackStatus::Paused {
                    self.engine.resume();
                    self.state.status = PlaybackStatus::Playing;
                    self.broadcast_state();
                } else {
                    self.play_next_internal(false).await?;
                }
            }
            SocketCommand::Next => {
                if let Some(ref current) = self.current_track_full {
                    let played_secs = self.engine.get_position().as_secs_f64();
                    if let Some(ref station) = self.state.station {
                        let event = RotorFeedbackEvent::skip(
                            &current.id,
                            played_secs,
                            self.current_batch_id.as_deref(),
                        );
                        let _ = self.client.send_rotor_feedback(station, &event).await;
                    }
                }
                let use_xfade = self.state.audio_settings.crossfade_enabled
                    && self.state.audio_settings.crossfade_on_manual_skip;
                self.play_next_internal(use_xfade).await?;
            }
            SocketCommand::Prev => {
                self.play_prev().await?;
            }
            SocketCommand::SetVolume { volume } => {
                self.engine.set_volume(volume);
                self.state.volume = self.engine.get_volume();
                self.config.audio.volume = self.state.volume;
                let _ = self.config.save();
                self.broadcast_state();
            }
            SocketCommand::ToggleLike => {
                self.toggle_like().await?;
            }
            SocketCommand::Like => {
                self.set_like(true).await?;
            }
            SocketCommand::Dislike => {
                self.set_like(false).await?;
                let use_xfade = self.state.audio_settings.crossfade_enabled
                    && self.state.audio_settings.crossfade_on_manual_skip;
                self.play_next_internal(use_xfade).await?;
            }
            SocketCommand::PlayVibe {
                station,
                diversity,
                mood_energy,
                language,
            } => {
                let station_id = station.unwrap_or_else(|| {
                    self.state
                        .station
                        .clone()
                        .unwrap_or_else(|| "user:onyourwave".to_string())
                });
                if let Some(d) = diversity {
                    self.state.vibe_settings.diversity = d;
                }
                if let Some(m) = mood_energy {
                    self.state.vibe_settings.mood_energy = m;
                }
                if let Some(l) = language {
                    self.state.vibe_settings.language = l;
                }
                self.start_station(&station_id).await?;
            }
            SocketCommand::SetVibeSettings {
                diversity,
                mood_energy,
                language,
            } => {
                let mut changed = false;
                if let Some(d) = diversity {
                    if self.state.vibe_settings.diversity != d {
                        self.state.vibe_settings.diversity = d;
                        changed = true;
                    }
                }
                if let Some(m) = mood_energy {
                    if self.state.vibe_settings.mood_energy != m {
                        self.state.vibe_settings.mood_energy = m;
                        changed = true;
                    }
                }
                if let Some(l) = language {
                    if self.state.vibe_settings.language != l {
                        self.state.vibe_settings.language = l;
                        changed = true;
                    }
                }
                if changed || self.state.status == PlaybackStatus::Stopped {
                    let station = self
                        .state
                        .station
                        .clone()
                        .unwrap_or_else(|| "user:onyourwave".to_string());
                    self.start_station(&station).await?;
                } else {
                    self.broadcast_state();
                }
            }
            SocketCommand::SetAudioSettings {
                crossfade_enabled,
                crossfade_duration,
                crossfade_on_manual_skip,
            } => {
                if let Some(enabled) = crossfade_enabled {
                    self.state.audio_settings.crossfade_enabled = enabled;
                    self.config.audio.crossfade_enabled = enabled;
                }
                if let Some(dur) = crossfade_duration {
                    let clamped = dur.clamp(1, 5);
                    self.state.audio_settings.crossfade_duration = clamped;
                    self.config.audio.crossfade_duration = clamped;
                }
                if let Some(on_skip) = crossfade_on_manual_skip {
                    self.state.audio_settings.crossfade_on_manual_skip = on_skip;
                    self.config.audio.crossfade_on_manual_skip = on_skip;
                }
                let _ = self.config.save();
                self.broadcast_state();
            }
            SocketCommand::GetStatus => {}
            SocketCommand::GetQueue => {}
        }

        Ok(self.state())
    }

    pub async fn start_station(&mut self, station_id: &str) -> Result<()> {
        self.state.station = Some(station_id.to_string());
        self.queue.clear();
        self.prefetched_next = None;
        self.fetch_more_station_tracks().await?;

        let event = RotorFeedbackEvent::radio_started(self.current_batch_id.as_deref());
        let _ = self.client.send_rotor_feedback(station_id, &event).await;

        self.play_next_internal(false).await?;
        Ok(())
    }

    pub async fn fetch_more_station_tracks(&mut self) -> Result<()> {
        let station = self
            .state
            .station
            .clone()
            .unwrap_or_else(|| "user:onyourwave".to_string());

        println!(
            "[yamarchy-daemon] Fetching tracks for station: {station} (diversity: {}, mood: {}, language: {})",
            self.state.vibe_settings.diversity,
            self.state.vibe_settings.mood_energy,
            self.state.vibe_settings.language
        );
        let batch: RotorBatchResponse = self
            .client
            .get_rotor_station_tracks(&station, Some(&self.state.vibe_settings))
            .await?;
        self.current_batch_id = batch.batch_id.clone();
        println!(
            "[yamarchy-daemon] Fetched batch {:?} with {} tracks",
            batch.batch_id,
            batch.sequence.len()
        );

        for item in batch.sequence {
            self.queue.push_back(item.track);
        }

        self.update_queue_summary();
        Ok(())
    }

    pub async fn play_next_internal(&mut self, crossfade: bool) -> Result<()> {
        if self.queue.is_empty() && self.prefetched_next.is_none() {
            if let Err(e) = self.fetch_more_station_tracks().await {
                eprintln!("[yamarchy-daemon] Failed to fetch station tracks: {e}");
            }
        }

        let next_item = if let Some(prefetched) = self.prefetched_next.take() {
            // Remove matching track from front of queue if present
            if self.queue.front().map(|t| &t.id) == Some(&prefetched.0.id) {
                self.queue.pop_front();
            }
            Some(prefetched)
        } else if let Some(track) = self.queue.pop_front() {
            match self.fetch_track_bytes(&track.id).await {
                Ok(bytes) => Some((track, bytes)),
                Err(e) => {
                    eprintln!(
                        "[yamarchy-daemon] Failed to fetch stream for track {}: {e}",
                        track.id
                    );
                    None
                }
            }
        } else {
            None
        };

        if let Some((track, audio_bytes)) = next_item {
            if let Some(prev) = self.current_track_full.take() {
                self.history.push(prev);
            }
            if let Err(e) = self.play_track_bytes(track, audio_bytes, crossfade).await {
                eprintln!("[yamarchy-daemon] Failed to play track: {e}");
            }
        } else {
            self.engine.stop();
            self.state.status = PlaybackStatus::Stopped;
            self.state.current_track = None;
            self.broadcast_state();
        }

        Ok(())
    }

    pub async fn play_prev(&mut self) -> Result<()> {
        if let Some(track) = self.history.pop() {
            if let Some(current) = self.current_track_full.take() {
                self.queue.push_front(current);
            }
            self.prefetched_next = None;
            let bytes = self.fetch_track_bytes(&track.id).await?;
            let use_xfade = self.state.audio_settings.crossfade_enabled
                && self.state.audio_settings.crossfade_on_manual_skip;
            self.play_track_bytes(track, bytes, use_xfade).await?;
        } else {
            self.engine.resume();
        }
        Ok(())
    }

    async fn fetch_track_bytes(&self, track_id: &str) -> Result<bytes::Bytes> {
        let stream_url = self.client.get_direct_stream_url(track_id).await?;
        println!("[yamarchy-daemon] Downloading stream from: {stream_url}");
        let audio_bytes = self.fetcher.fetch_audio_bytes(&stream_url).await?;
        println!("[yamarchy-daemon] Downloaded {} bytes.", audio_bytes.len());
        Ok(audio_bytes)
    }

    async fn prefetch_next_if_needed(&mut self) {
        if self.is_prefetching || self.prefetched_next.is_some() {
            return;
        }

        if self.queue.is_empty() {
            let _ = self.fetch_more_station_tracks().await;
        }

        if let Some(next_track) = self.queue.front() {
            let track_id = next_track.id.clone();
            let track_clone = next_track.clone();
            println!(
                "[yamarchy-daemon] Prefetching next track: {} (ID: {})",
                track_clone.display_title(),
                track_id
            );
            self.is_prefetching = true;

            match self.fetch_track_bytes(&track_id).await {
                Ok(bytes) => {
                    println!(
                        "[yamarchy-daemon] Successfully prefetched next track: {}",
                        track_clone.display_title()
                    );
                    self.prefetched_next = Some((track_clone, bytes));
                }
                Err(e) => {
                    eprintln!(
                        "[yamarchy-daemon] Prefetch failed for track {}: {e}",
                        track_id
                    );
                }
            }
            self.is_prefetching = false;
        }
    }

    async fn play_track_bytes(
        &mut self,
        track: Track,
        audio_bytes: bytes::Bytes,
        crossfade: bool,
    ) -> Result<()> {
        let title = track.display_title();
        let artist = track.display_artists();
        println!(
            "[yamarchy-daemon] Now loading track: {artist} - {title} (ID: {})",
            track.id
        );

        let summary = TrackSummary {
            id: track.id.clone(),
            title: title.clone(),
            artist: artist.clone(),
            album: track
                .albums
                .first()
                .map(|a| a.title.clone())
                .unwrap_or_default(),
            duration_ms: track.duration_ms,
            cover_url: track.cover_url(400),
            liked: track.liked.unwrap_or(false),
        };

        self.state.current_track = Some(summary);
        self.state.duration_ms = track.duration_ms;
        self.state.position_ms = 0;
        self.state.liked = track.liked.unwrap_or(false);

        if crossfade && self.state.audio_settings.crossfade_enabled {
            let xfade_duration = Duration::from_secs(
                self.state.audio_settings.crossfade_duration.clamp(1, 5) as u64,
            );
            println!(
                "[yamarchy-daemon] Crossfading to track with duration {:?}",
                xfade_duration
            );
            self.engine
                .crossfade_to_bytes(audio_bytes, xfade_duration)?;
        } else {
            self.engine.play_bytes(audio_bytes)?;
        }

        self.state.status = PlaybackStatus::Playing;
        println!("[yamarchy-daemon] Playback active: {artist} - {title}");

        // Feedback event
        if let Some(ref station) = self.state.station {
            let event =
                RotorFeedbackEvent::track_started(&track.id, self.current_batch_id.as_deref());
            let _ = self.client.send_rotor_feedback(station, &event).await;
        }

        self.current_track_full = Some(track);
        self.update_queue_summary();
        self.broadcast_state();
        Ok(())
    }

    pub async fn toggle_like(&mut self) -> Result<()> {
        let new_state = !self.state.liked;
        self.set_like(new_state).await
    }

    pub async fn set_like(&mut self, like: bool) -> Result<()> {
        if let (Some(uid), Some(ref track)) = (self.user_uid, &self.current_track_full) {
            if like {
                self.client.add_like(uid, &track.id).await?;
            } else {
                self.client.remove_like(uid, &track.id).await?;
            }
            self.state.liked = like;
            if let Some(ref mut summary) = self.state.current_track {
                summary.liked = like;
            }
            self.broadcast_state();
        }
        Ok(())
    }

    pub async fn tick(&mut self) {
        self.engine.tick();

        if self.state.status == PlaybackStatus::Playing {
            let pos = self.engine.get_position();
            self.state.position_ms = pos.as_millis() as u64;
            let duration_ms = self.state.duration_ms;
            let remaining_ms = duration_ms.saturating_sub(self.state.position_ms);

            let crossfade_enabled = self.state.audio_settings.crossfade_enabled;
            let crossfade_dur_sec = self.state.audio_settings.crossfade_duration.clamp(1, 5);
            let crossfade_ms = (crossfade_dur_sec as u64) * 1000;

            // Trigger prefetch when track approaches end
            if duration_ms > 0 && remaining_ms <= (crossfade_ms + 12_000) {
                self.prefetch_next_if_needed().await;
            }

            // Crossfade transition trigger
            if crossfade_enabled
                && duration_ms > 0
                && remaining_ms <= crossfade_ms
                && remaining_ms > 0
            {
                if let Some(ref current) = self.current_track_full {
                    let played_secs = pos.as_secs_f64();
                    if let Some(ref station) = self.state.station {
                        let event = RotorFeedbackEvent::track_finished(
                            &current.id,
                            played_secs,
                            self.current_batch_id.as_deref(),
                        );
                        let _ = self.client.send_rotor_feedback(station, &event).await;
                    }
                }
                let _ = self.play_next_internal(true).await;
            } else if (!crossfade_enabled || self.engine.is_empty())
                && duration_ms > 0
                && (self.engine.is_empty()
                    || self.state.position_ms >= duration_ms.saturating_sub(1500))
            {
                // Track finished naturally without crossfade
                if let Some(ref current) = self.current_track_full {
                    let played_secs = pos.as_secs_f64();
                    if let Some(ref station) = self.state.station {
                        let event = RotorFeedbackEvent::track_finished(
                            &current.id,
                            played_secs,
                            self.current_batch_id.as_deref(),
                        );
                        let _ = self.client.send_rotor_feedback(station, &event).await;
                    }
                }
                let _ = self.play_next_internal(false).await;
            }
        }
    }

    fn update_queue_summary(&mut self) {
        self.state.queue = self
            .queue
            .iter()
            .take(20)
            .map(|t| TrackSummary {
                id: t.id.clone(),
                title: t.display_title(),
                artist: t.display_artists(),
                album: t
                    .albums
                    .first()
                    .map(|a| a.title.clone())
                    .unwrap_or_default(),
                duration_ms: t.duration_ms,
                cover_url: t.cover_url(200),
                liked: t.liked.unwrap_or(false),
            })
            .collect();
    }

    fn broadcast_state(&self) {
        let _ = self.state_tx.send(self.state());
    }
}
