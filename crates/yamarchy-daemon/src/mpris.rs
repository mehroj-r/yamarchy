use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::connection::Connection;
use zbus::interface;
use zbus::zvariant::Value;

use crate::service::PlayerService;
use crate::socket::SocketCommand;

pub struct MprisRoot;

#[interface(interface = "org.mpris.MediaPlayer2")]
impl MprisRoot {
    async fn raise(&self) {}
    async fn quit(&self) {}

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> &str {
        "Yamarchy Yandex Music"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        vec!["yandexmusic".to_string()]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        vec!["audio/mpeg".to_string()]
    }
}

pub struct MprisPlayer {
    service: Arc<Mutex<PlayerService>>,
}

impl MprisPlayer {
    pub fn new(service: Arc<Mutex<PlayerService>>) -> Self {
        Self { service }
    }
}

#[interface(interface = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    async fn next(&self) {
        let mut svc = self.service.lock().await;
        let _ = svc.handle_command(SocketCommand::Next).await;
    }

    async fn previous(&self) {
        let mut svc = self.service.lock().await;
        let _ = svc.handle_command(SocketCommand::Prev).await;
    }

    async fn pause(&self) {
        let mut svc = self.service.lock().await;
        let _ = svc.handle_command(SocketCommand::Pause).await;
    }

    async fn play_pause(&self) {
        let mut svc = self.service.lock().await;
        let _ = svc.handle_command(SocketCommand::Toggle).await;
    }

    async fn stop(&self) {
        let mut svc = self.service.lock().await;
        let _ = svc.handle_command(SocketCommand::Pause).await;
    }

    async fn play(&self) {
        let mut svc = self.service.lock().await;
        let _ = svc.handle_command(SocketCommand::Play).await;
    }

    #[zbus(property)]
    async fn playback_status(&self) -> String {
        let svc = self.service.lock().await;
        match svc.state().status {
            yamarchy_core::models::PlaybackStatus::Playing => "Playing".to_string(),
            yamarchy_core::models::PlaybackStatus::Paused => "Paused".to_string(),
            yamarchy_core::models::PlaybackStatus::Stopped => "Stopped".to_string(),
        }
    }

    #[zbus(property)]
    async fn volume(&self) -> f64 {
        let svc = self.service.lock().await;
        svc.state().volume as f64
    }

    #[zbus(property)]
    async fn set_volume(&self, volume: f64) {
        let mut svc = self.service.lock().await;
        let _ = svc
            .handle_command(SocketCommand::SetVolume {
                volume: volume as f32,
            })
            .await;
    }

    #[zbus(property)]
    async fn position(&self) -> i64 {
        let svc = self.service.lock().await;
        (svc.state().position_ms * 1000) as i64
    }

    #[zbus(property)]
    async fn metadata(&self) -> HashMap<String, Value<'_>> {
        let svc = self.service.lock().await;
        let mut map = HashMap::new();
        let state = svc.state();

        if let Some(track) = state.current_track {
            map.insert(
                "mpris:trackid".to_string(),
                Value::from(format!("/org/yamarchy/tracks/{}", track.id)),
            );
            map.insert(
                "mpris:length".to_string(),
                Value::from((track.duration_ms * 1000) as i64),
            );
            map.insert("xesam:title".to_string(), Value::from(track.title));
            map.insert("xesam:artist".to_string(), Value::from(vec![track.artist]));
            map.insert("xesam:album".to_string(), Value::from(track.album));
            if let Some(cover) = track.cover_url {
                map.insert("mpris:artUrl".to_string(), Value::from(cover));
            }
        }

        map
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
}

pub async fn start_mpris_server(service: Arc<Mutex<PlayerService>>) -> anyhow::Result<Connection> {
    let connection = Connection::session().await?;
    connection
        .object_server()
        .at("/org/mpris/MediaPlayer2", MprisRoot)
        .await?;
    connection
        .object_server()
        .at("/org/mpris/MediaPlayer2", MprisPlayer::new(service))
        .await?;

    connection
        .request_name("org.mpris.MediaPlayer2.yamarchy")
        .await?;

    Ok(connection)
}
