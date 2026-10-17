use anyhow::{Context, Result};
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct AudioStreamFetcher {
    client: reqwest::Client,
}

impl Default for AudioStreamFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioStreamFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder().build().unwrap_or_default(),
        }
    }

    pub async fn fetch_audio_bytes(&self, url: &str) -> Result<Bytes> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch audio stream")?;

        if !resp.status().is_success() {
            anyhow::bail!("Audio stream fetch returned HTTP {}", resp.status());
        }

        let bytes = resp.bytes().await.context("Failed to read audio bytes")?;
        Ok(bytes)
    }
}
