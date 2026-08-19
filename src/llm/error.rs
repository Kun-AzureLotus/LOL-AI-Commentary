use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("missing required environment variable: {name}")]
    MissingEnv { name: &'static str },

    #[error("invalid LLM base URL: {0}")]
    InvalidBaseUrl(String),

    #[error("failed to build LLM HTTP client")]
    ClientBuild {
        #[source]
        source: reqwest::Error,
    },

    #[error("LLM request timed out at {url}")]
    Timeout {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("failed to send LLM request to {url}")]
    RequestFailed {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("LLM returned HTTP {status} for {url}: {body}")]
    HttpStatus {
        url: String,
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("failed to decode LLM response from {url}")]
    Decode {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("LLM response did not contain a commentary message")]
    EmptyResponse,
}
