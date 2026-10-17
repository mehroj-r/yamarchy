use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub result: Option<T>,
    pub error: Option<ApiError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub name: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountStatus {
    pub account: Option<AccountInfo>,
    pub permissions: Option<Permissions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub uid: Option<u64>,
    pub login: Option<String>,
    #[serde(rename = "fullName")]
    pub full_name: Option<String>,
    #[serde(rename = "secondName")]
    pub second_name: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "serviceAvailable")]
    pub service_available: Option<bool>,
    pub now: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub until: Option<String>,
    pub values: Option<Vec<String>>,
    #[serde(rename = "default")]
    pub default_perms: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Track {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub albums: Vec<Album>,
    #[serde(rename = "durationMs", default)]
    pub duration_ms: u64,
    #[serde(rename = "coverUri", default)]
    pub cover_uri: Option<String>,
    #[serde(default)]
    pub available: Option<bool>,
    #[serde(default)]
    pub liked: Option<bool>,
}

impl Track {
    pub fn display_artists(&self) -> String {
        if self.artists.is_empty() {
            "Unknown Artist".to_string()
        } else {
            self.artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    pub fn display_title(&self) -> String {
        if let Some(ref version) = self.version {
            if !version.is_empty() {
                return format!("{} ({})", self.title, version);
            }
        }
        self.title.clone()
    }

    pub fn cover_url(&self, size: u32) -> Option<String> {
        self.cover_uri.as_ref().map(|uri| {
            let replaced = uri.replace("%%", &format!("{size}x{size}"));
            if replaced.starts_with("http://") || replaced.starts_with("https://") {
                replaced
            } else {
                format!("https://{replaced}")
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Artist {
    pub id: Option<serde_json::Value>,
    pub name: String,
    #[serde(default)]
    pub various: Option<bool>,
    #[serde(default)]
    pub composer: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Album {
    pub id: Option<serde_json::Value>,
    pub title: String,
    #[serde(default)]
    pub year: Option<u32>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(rename = "coverUri", default)]
    pub cover_uri: Option<String>,
    #[serde(rename = "trackCount", default)]
    pub track_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadInfo {
    pub codec: String,
    #[serde(default)]
    pub bitrate_in_kbps: u32,
    #[serde(default)]
    pub gain: bool,
    #[serde(default)]
    pub preview: bool,
    pub download_info_url: String,
    #[serde(default)]
    pub direct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotorStation {
    pub id: RotorStationId,
    pub name: Option<String>,
    pub icon: Option<RotorIcon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotorStationId {
    #[serde(rename = "type")]
    pub station_type: String,
    pub tag: String,
}

impl RotorStationId {
    pub fn to_station_string(&self) -> String {
        format!("{}:{}", self.station_type, self.tag)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotorIcon {
    #[serde(rename = "backgroundColor")]
    pub background_color: Option<String>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotorBatchResponse {
    pub sequence: Vec<RotorSequenceItem>,
    #[serde(rename = "batchId")]
    pub batch_id: Option<String>,
    #[serde(rename = "pumpkin")]
    pub pumpkin: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotorSequenceItem {
    pub track: Track,
    #[serde(default)]
    pub liked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLikesTracks {
    #[serde(rename = "library")]
    pub library: Option<LikesLibrary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LikesLibrary {
    pub uid: Option<u64>,
    #[serde(rename = "tracks")]
    pub tracks: Option<Vec<LikedTrackItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LikedTrackItem {
    pub id: String,
    #[serde(rename = "albumId")]
    pub album_id: Option<String>,
    pub timestamp: Option<String>,
}

fn default_diversity() -> String {
    "default".to_string()
}

fn default_mood_energy() -> String {
    "all".to_string()
}

fn default_language() -> String {
    "any".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VibeSettings {
    #[serde(default = "default_diversity")]
    pub diversity: String,
    #[serde(default = "default_mood_energy")]
    pub mood_energy: String,
    #[serde(default = "default_language")]
    pub language: String,
}

impl Default for VibeSettings {
    fn default() -> Self {
        Self {
            diversity: default_diversity(),
            mood_energy: default_mood_energy(),
            language: default_language(),
        }
    }
}

fn default_crossfade_duration() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioSettings {
    #[serde(default)]
    pub crossfade_enabled: bool,
    #[serde(default = "default_crossfade_duration")]
    pub crossfade_duration: u32,
    #[serde(default)]
    pub crossfade_on_manual_skip: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            crossfade_enabled: false,
            crossfade_duration: default_crossfade_duration(),
            crossfade_on_manual_skip: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub status: PlaybackStatus,
    pub current_track: Option<TrackSummary>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub station: Option<String>,
    #[serde(default)]
    pub vibe_settings: VibeSettings,
    #[serde(default)]
    pub audio_settings: AudioSettings,
    pub queue: Vec<TrackSummary>,
    pub liked: bool,
    pub shuffle: bool,
    pub repeat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackSummary {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub cover_url: Option<String>,
    pub liked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_settings_defaults() {
        let settings = AudioSettings::default();
        assert!(!settings.crossfade_enabled);
        assert_eq!(settings.crossfade_duration, 3);
        assert!(!settings.crossfade_on_manual_skip);
    }

    #[test]
    fn test_audio_settings_json_roundtrip() {
        let json =
            r#"{"crossfade_enabled":true,"crossfade_duration":4,"crossfade_on_manual_skip":true}"#;
        let parsed: AudioSettings = serde_json::from_str(json).unwrap();
        assert!(parsed.crossfade_enabled);
        assert_eq!(parsed.crossfade_duration, 4);
        assert!(parsed.crossfade_on_manual_skip);

        let serialized = serde_json::to_string(&parsed).unwrap();
        assert_eq!(serialized, json);
    }
}
