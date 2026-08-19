use thiserror::Error;

#[derive(Debug, Error)]
pub enum RiotLiveClientError {
    #[error("failed to connect to Riot Live Client API at {url}")]
    ConnectionFailed {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Riot Live Client API request timed out at {url}")]
    Timeout {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Riot Live Client API returned HTTP {status} for {url}")]
    HttpStatus {
        url: String,
        status: reqwest::StatusCode,
    },

    #[error("failed to decode Riot Live Client API response from {url}")]
    Decode {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("failed to build Riot Live Client HTTP client")]
    ClientBuild {
        #[source]
        source: reqwest::Error,
    },

    #[error("invalid Riot Live Client base URL: {0}")]
    InvalidBaseUrl(String),
}
