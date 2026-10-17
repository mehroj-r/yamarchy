use crate::models::{AccountStatus, ApiResponse};
use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, AUTHORIZATION, USER_AGENT};

pub const DEFAULT_USER_AGENT: &str = "YandexMusicAndroid/24022411 (Android 14; Pixel 8 Pro)";

#[derive(Debug, Clone)]
pub struct AuthManager {
    token: String,
    user_agent: String,
}

impl AuthManager {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            user_agent: DEFAULT_USER_AGENT.to_string(),
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn build_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_str(&self.user_agent)?);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("ru, en;q=0.9"));
        if !self.token.is_empty() {
            let auth_val = format!("OAuth {}", self.token.trim());
            headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_val)?);
        }
        Ok(headers)
    }

    pub async fn validate_token(&self, client: &reqwest::Client) -> Result<AccountStatus> {
        let url = "https://api.music.yandex.net/account/status";
        let headers = self.build_headers()?;

        let resp = client
            .get(url)
            .headers(headers)
            .send()
            .await
            .context("Failed to send token validation request")?;

        if !resp.status().is_success() {
            anyhow::bail!("Yandex Music API returned HTTP {}", resp.status());
        }

        let body: ApiResponse<AccountStatus> = resp
            .json()
            .await
            .context("Failed to parse account status response")?;

        if let Some(err) = body.error {
            anyhow::bail!(
                "Yandex API error: {} - {}",
                err.name.unwrap_or_default(),
                err.message.unwrap_or_default()
            );
        }

        body.result
            .context("Empty result in account status response")
    }
}
