use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, Mutex};
use yamarchy_core::models::PlayerState;

use crate::service::PlayerService;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum SocketCommand {
    Play,
    Pause,
    Toggle,
    Next,
    Prev,
    ToggleLike,
    Like,
    Dislike,
    SetVolume {
        volume: f32,
    },
    PlayVibe {
        #[serde(default)]
        station: Option<String>,
        #[serde(default)]
        diversity: Option<String>,
        #[serde(default)]
        mood_energy: Option<String>,
        #[serde(default)]
        language: Option<String>,
    },
    SetVibeSettings {
        #[serde(default)]
        diversity: Option<String>,
        #[serde(default)]
        mood_energy: Option<String>,
        #[serde(default)]
        language: Option<String>,
    },
    SetAudioSettings {
        #[serde(default)]
        crossfade_enabled: Option<bool>,
        #[serde(default)]
        crossfade_duration: Option<u32>,
        #[serde(default)]
        crossfade_on_manual_skip: Option<bool>,
    },
    GetStatus,
    GetQueue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<PlayerState>,
}

impl SocketResponse {
    pub fn ok_with_state(state: PlayerState) -> Self {
        Self {
            status: "ok".to_string(),
            message: None,
            state: Some(state),
        }
    }

    #[allow(dead_code)]
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
            message: None,
            state: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            message: Some(msg.into()),
            state: None,
        }
    }
}

pub struct SocketServer {
    listener: UnixListener,
    service: Arc<Mutex<PlayerService>>,
    state_broadcast_tx: broadcast::Sender<PlayerState>,
}

impl SocketServer {
    pub fn bind(
        socket_path: &Path,
        service: Arc<Mutex<PlayerService>>,
        state_broadcast_tx: broadcast::Sender<PlayerState>,
    ) -> Result<Self> {
        if socket_path.exists() {
            let _ = std::fs::remove_file(socket_path);
        }
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(socket_path)
            .with_context(|| format!("Failed to bind unix socket at {}", socket_path.display()))?;

        Ok(Self {
            listener,
            service,
            state_broadcast_tx,
        })
    }

    pub async fn run(self) -> Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, _)) => {
                    let service = Arc::clone(&self.service);
                    let mut state_rx = self.state_broadcast_tx.subscribe();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, service, &mut state_rx).await {
                            eprintln!("[yamarchy-daemon] client handler error: {e}");
                        }
                    });
                }
                Err(e) => {
                    eprintln!("[yamarchy-daemon] unix socket accept error: {e}");
                }
            }
        }
    }
}

async fn handle_client(
    stream: UnixStream,
    service: Arc<Mutex<PlayerService>>,
    state_rx: &mut broadcast::Receiver<PlayerState>,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        tokio::select! {
            result = reader.read_line(&mut line) => {
                let bytes_read = result?;
                if bytes_read == 0 {
                    break; // Connection closed
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    line.clear();
                    continue;
                }

                let response = match serde_json::from_str::<SocketCommand>(trimmed) {
                    Ok(cmd) => {
                        let mut svc = service.lock().await;
                        match svc.handle_command(cmd).await {
                            Ok(state) => SocketResponse::ok_with_state(state),
                            Err(e) => SocketResponse::error(e.to_string()),
                        }
                    }
                    Err(e) => SocketResponse::error(format!("Invalid command: {e}")),
                };

                let mut resp_str = serde_json::to_string(&response)?;
                resp_str.push('\n');
                writer.write_all(resp_str.as_bytes()).await?;
                writer.flush().await?;
                line.clear();
            }
            Ok(state) = state_rx.recv() => {
                let broadcast_msg = SocketResponse::ok_with_state(state);
                let mut resp_str = serde_json::to_string(&broadcast_msg)?;
                resp_str.push('\n');
                let _ = writer.write_all(resp_str.as_bytes()).await;
                let _ = writer.flush().await;
            }
        }
    }

    Ok(())
}
