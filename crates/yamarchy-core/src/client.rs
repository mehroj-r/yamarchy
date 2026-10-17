use crate::auth::AuthManager;
use crate::download::{generate_direct_download_url, parse_download_info_xml};
use crate::models::{
    AccountStatus, ApiResponse, DownloadInfo, RotorBatchResponse, Track, UserLikesTracks,
    VibeSettings,
};
use crate::rotor::RotorFeedbackEvent;
use anyhow::{Context, Result};
use reqwest::header::HeaderMap;

const BASE_URL: &str = "https://api.music.yandex.net";

#[derive(Debug, Clone)]
pub struct YandexMusicClient {
    client: reqwest::Client,
    auth: AuthManager,
}

impl YandexMusicClient {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder().build().unwrap_or_default(),
            auth: AuthManager::new(token),
        }
    }

    pub fn auth(&self) -> &AuthManager {
        &self.auth
    }

    fn headers(&self) -> Result<HeaderMap> {
        self.auth.build_headers()
    }

    pub async fn get_account_status(&self) -> Result<AccountStatus> {
        self.auth.validate_token(&self.client).await
    }

    pub async fn get_track(&self, track_id: &str) -> Result<Track> {
        let url = format!("{BASE_URL}/tracks/{track_id}");
        let resp = self
            .client
            .get(&url)
            .headers(self.headers()?)
            .send()
            .await
            .context("Failed to get track metadata")?;

        let body: ApiResponse<Vec<Track>> = resp.json().await?;
        let tracks = body.result.context("Empty tracks list returned")?;
        tracks
            .into_iter()
            .next()
            .context("No track found with specified ID")
    }

    pub async fn get_download_info(&self, track_id: &str) -> Result<Vec<DownloadInfo>> {
        let url = format!("{BASE_URL}/tracks/{track_id}/download-info");
        let resp = self
            .client
            .get(&url)
            .headers(self.headers()?)
            .send()
            .await
            .context("Failed to get track download info")?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("Get download info failed ({status}): {text}");
        }

        let body: ApiResponse<Vec<DownloadInfo>> = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse download info JSON. Raw: {text}"))?;
        body.result.context("Empty download info result")
    }

    pub async fn get_direct_stream_url(&self, track_id: &str) -> Result<String> {
        let infos = self.get_download_info(track_id).await?;
        // Prefer mp3 codec with highest bitrate
        let best_info = infos
            .iter()
            .filter(|i| i.codec == "mp3")
            .max_by_key(|i| i.bitrate_in_kbps)
            .or_else(|| infos.first())
            .context("No valid download info for track")?;

        let xml_resp = self
            .client
            .get(&best_info.download_info_url)
            .send()
            .await
            .context("Failed to fetch download XML")?;

        if !xml_resp.status().is_success() {
            let status = xml_resp.status();
            let text = xml_resp.text().await.unwrap_or_default();
            anyhow::bail!("Fetch download XML failed ({status}): {text}");
        }

        let xml_text = xml_resp.text().await?;
        let parsed_xml = parse_download_info_xml(&xml_text)?;
        generate_direct_download_url(&parsed_xml)
    }

    pub async fn get_rotor_station_tracks(
        &self,
        station: &str,
        settings: Option<&VibeSettings>,
    ) -> Result<RotorBatchResponse> {
        let mut url =
            format!("{BASE_URL}/rotor/station/{station}/tracks?include-tracks-in-storage=true");
        if let Some(s) = settings {
            if !s.diversity.is_empty() && s.diversity != "default" {
                url.push_str(&format!("&diversity={}", s.diversity));
            }
            if !s.mood_energy.is_empty() && s.mood_energy != "all" {
                url.push_str(&format!("&moodEnergy={}", s.mood_energy));
            }
            if !s.language.is_empty() && s.language != "any" {
                url.push_str(&format!("&language={}", s.language));
            }
        }
        let resp = self
            .client
            .get(&url)
            .headers(self.headers()?)
            .send()
            .await
            .context("Failed to fetch rotor station tracks")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Rotor station request failed ({status}): {text}");
        }

        let body: ApiResponse<RotorBatchResponse> = resp.json().await?;
        body.result.context("Empty rotor batch result")
    }

    pub async fn send_rotor_feedback(
        &self,
        station: &str,
        event: &RotorFeedbackEvent,
    ) -> Result<()> {
        let url = if let Some(ref batch_id) = event.batch_id {
            format!("{BASE_URL}/rotor/station/{station}/feedback?batch-id={batch_id}")
        } else {
            format!("{BASE_URL}/rotor/station/{station}/feedback")
        };
        let resp = self
            .client
            .post(&url)
            .headers(self.headers()?)
            .json(event)
            .send()
            .await
            .context("Failed to send rotor feedback")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            eprintln!("[yamarchy] rotor feedback warning ({status}): {text}");
        }
        Ok(())
    }

    pub async fn get_liked_tracks(&self, uid: u64) -> Result<UserLikesTracks> {
        let url = format!("{BASE_URL}/users/{uid}/likes/tracks");
        let resp = self
            .client
            .get(&url)
            .headers(self.headers()?)
            .send()
            .await
            .context("Failed to fetch liked tracks")?;

        let body: ApiResponse<UserLikesTracks> = resp.json().await?;
        body.result.context("Empty liked tracks result")
    }

    pub async fn add_like(&self, uid: u64, track_id: &str) -> Result<()> {
        let url = format!("{BASE_URL}/users/{uid}/likes/tracks/add-multiple");
        let params = [("track-ids", track_id)];
        let resp = self
            .client
            .post(&url)
            .headers(self.headers()?)
            .form(&params)
            .send()
            .await
            .context("Failed to add like")?;

        if !resp.status().is_success() {
            anyhow::bail!("Add like returned HTTP {}", resp.status());
        }
        Ok(())
    }

    pub async fn remove_like(&self, uid: u64, track_id: &str) -> Result<()> {
        let url = format!("{BASE_URL}/users/{uid}/likes/tracks/remove");
        let params = [("track-ids", track_id)];
        let resp = self
            .client
            .post(&url)
            .headers(self.headers()?)
            .form(&params)
            .send()
            .await
            .context("Failed to remove like")?;

        if !resp.status().is_success() {
            anyhow::bail!("Remove like returned HTTP {}", resp.status());
        }
        Ok(())
    }
}
