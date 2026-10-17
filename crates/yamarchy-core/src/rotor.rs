use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotorFeedbackEvent {
    #[serde(rename = "type")]
    pub event_type: RotorEventType,
    pub timestamp: String,
    #[serde(rename = "trackId", skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,
    #[serde(rename = "totalPlayedSeconds", skip_serializing_if = "Option::is_none")]
    pub total_played_seconds: Option<f64>,
    #[serde(rename = "batchId", skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RotorEventType {
    RadioStarted,
    TrackStarted,
    TrackFinished,
    Skip,
    Like,
    Dislike,
}

impl RotorFeedbackEvent {
    pub fn track_started(track_id: &str, batch_id: Option<&str>) -> Self {
        Self {
            event_type: RotorEventType::TrackStarted,
            timestamp: chrono_now_iso(),
            track_id: Some(track_id.to_string()),
            total_played_seconds: None,
            batch_id: batch_id.map(ToString::to_string),
        }
    }

    pub fn track_finished(track_id: &str, played_secs: f64, batch_id: Option<&str>) -> Self {
        Self {
            event_type: RotorEventType::TrackFinished,
            timestamp: chrono_now_iso(),
            track_id: Some(track_id.to_string()),
            total_played_seconds: Some(played_secs),
            batch_id: batch_id.map(ToString::to_string),
        }
    }

    pub fn skip(track_id: &str, played_secs: f64, batch_id: Option<&str>) -> Self {
        Self {
            event_type: RotorEventType::Skip,
            timestamp: chrono_now_iso(),
            track_id: Some(track_id.to_string()),
            total_played_seconds: Some(played_secs),
            batch_id: batch_id.map(ToString::to_string),
        }
    }

    pub fn radio_started(batch_id: Option<&str>) -> Self {
        Self {
            event_type: RotorEventType::RadioStarted,
            timestamp: chrono_now_iso(),
            track_id: None,
            total_played_seconds: None,
            batch_id: batch_id.map(ToString::to_string),
        }
    }
}

fn chrono_now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
