use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use yamarchy_core::models::PlayerState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum CliCommand {
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
    pub message: Option<String>,
    pub state: Option<PlayerState>,
}

pub fn default_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("yamarchy.sock")
    } else {
        PathBuf::from("/tmp/yamarchy.sock")
    }
}

pub async fn send_command(socket_path: &Path, command: CliCommand) -> Result<SocketResponse> {
    let stream = UnixStream::connect(socket_path).await.with_context(|| {
        format!(
            "Failed to connect to yamarchy daemon socket at {}. Is yamarchy daemon running?",
            socket_path.display()
        )
    })?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut cmd_str = serde_json::to_string(&command)?;
    cmd_str.push('\n');

    writer.write_all(cmd_str.as_bytes()).await?;
    writer.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response: SocketResponse = serde_json::from_str(line.trim())
        .with_context(|| format!("Failed to parse response from daemon: {line}"))?;

    Ok(response)
}
