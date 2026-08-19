use std::time::Duration;

use reqwest::{Client, Url};

use super::{AllGameData, RiotLiveClientError};

const DEFAULT_BASE_URL: &str = "https://127.0.0.1:2999";
const ALL_GAME_DATA_PATH: &str = "/liveclientdata/allgamedata";

#[derive(Debug, Clone)]
pub struct RiotLiveClientConfig {
    pub base_url: String,
    pub timeout: Duration,
    pub accept_invalid_certs: bool,
}

impl Default for RiotLiveClientConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: Duration::from_secs(3),
            accept_invalid_certs: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RiotLiveClient {
    http: Client,
    base_url: Url,
}

impl RiotLiveClient {
    pub fn new(config: RiotLiveClientConfig) -> Result<Self, RiotLiveClientError> {
        let base_url = Url::parse(&config.base_url)
            .map_err(|_| RiotLiveClientError::InvalidBaseUrl(config.base_url.clone()))?;

        let http = Client::builder()
            .timeout(config.timeout)
            .danger_accept_invalid_certs(config.accept_invalid_certs)
            .build()
            .map_err(|source| RiotLiveClientError::ClientBuild { source })?;

        Ok(Self { http, base_url })
    }

    pub fn local() -> Result<Self, RiotLiveClientError> {
        Self::new(RiotLiveClientConfig::default())
    }

    pub async fn get_all_game_data(&self) -> Result<AllGameData, RiotLiveClientError> {
        let url = self.all_game_data_url()?;
        let url_string = url.to_string();

        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|source| map_request_error(url_string.clone(), source))?;

        let status = response.status();
        if !status.is_success() {
            return Err(RiotLiveClientError::HttpStatus {
                url: url_string,
                status,
            });
        }

        response
            .json::<AllGameData>()
            .await
            .map_err(|source| RiotLiveClientError::Decode {
                url: url_string,
                source,
            })
    }

    fn all_game_data_url(&self) -> Result<Url, RiotLiveClientError> {
        self.base_url
            .join(ALL_GAME_DATA_PATH)
            .map_err(|_| RiotLiveClientError::InvalidBaseUrl(self.base_url.to_string()))
    }
}

fn map_request_error(url: String, source: reqwest::Error) -> RiotLiveClientError {
    if source.is_timeout() {
        RiotLiveClientError::Timeout { url, source }
    } else {
        RiotLiveClientError::ConnectionFailed { url, source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_targets_local_live_client() {
        let config = RiotLiveClientConfig::default();

        assert_eq!(config.base_url, "https://127.0.0.1:2999");
        assert_eq!(config.timeout, Duration::from_secs(3));
        assert!(config.accept_invalid_certs);
    }
}
