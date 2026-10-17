use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub audio: AudioConfig,
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub vibe: VibeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    pub oauth_token: String,
}

fn default_quality() -> String {
    "high".to_string()
}

fn default_volume() -> f32 {
    0.8
}

fn default_cache_dir() -> String {
    "~/.cache/yamarchy".to_string()
}

fn default_crossfade_duration() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    #[serde(default = "default_quality")]
    pub quality: String,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    #[serde(default)]
    pub crossfade_enabled: bool,
    #[serde(default = "default_crossfade_duration")]
    pub crossfade_duration: u32,
    #[serde(default)]
    pub crossfade_on_manual_skip: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            quality: default_quality(),
            volume: default_volume(),
            cache_dir: default_cache_dir(),
            crossfade_enabled: false,
            crossfade_duration: default_crossfade_duration(),
            crossfade_on_manual_skip: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket_path: Option<String>,
    pub enable_mpris: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: None,
            enable_mpris: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibeConfig {
    pub station: String,
    pub diversity: String,
    pub mood_energy: String,
    pub language: String,
}

impl Default for VibeConfig {
    fn default() -> Self {
        Self {
            station: "user:onyourwave".to_string(),
            diversity: "default".to_string(),
            mood_energy: "all".to_string(),
            language: "any".to_string(),
        }
    }
}

impl Config {
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine user config directory")?
            .join("yamarchy");
        Ok(config_dir.join("config.toml"))
    }

    pub fn socket_path(&self) -> PathBuf {
        if let Some(ref path) = self.daemon.socket_path {
            return PathBuf::from(shellexpand(path));
        }
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            PathBuf::from(runtime_dir).join("yamarchy.sock")
        } else {
            PathBuf::from("/tmp/yamarchy.sock")
        }
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            // Check legacy or fallback config from yandex-music-tui if exists
            if let Some(config_dir) = dirs::config_dir() {
                let ym_tui_path = config_dir.join("yandex-music-tui").join("config.yaml");
                if ym_tui_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&ym_tui_path) {
                        if let Some(token) = extract_yaml_token(&content) {
                            let mut cfg = Config::default();
                            cfg.auth.oauth_token = token;
                            let _ = cfg.save();
                            return Ok(cfg);
                        }
                    }
                }
            }
            return Ok(Config::default());
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config at {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        write_private(&path, &content)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;
        Ok(())
    }
}

/// Writes `content` to `path` readable only by the owner (0600), since the
/// config holds the OAuth token.
fn write_private(path: &std::path::Path, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    // `mode` only applies on creation; tighten files written by older versions.
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(content.as_bytes())
}

fn shellexpand(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}/{}", home.display(), stripped);
        }
    }
    path.to_string()
}

fn extract_yaml_token(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("oauth_token:") {
            let parts: Vec<&str> = trimmed.split(':').collect();
            if parts.len() >= 2 {
                let val = parts[1].trim().trim_matches('"').trim_matches('\'');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}
