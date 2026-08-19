use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObsVisionError {
    #[error("missing required environment variable: {name}")]
    MissingEnv { name: &'static str },

    #[error("failed to connect to OBS WebSocket at {url}")]
    Connect {
        url: String,
        #[source]
        source: tokio_tungstenite::tungstenite::Error,
    },

    #[error("failed to send OBS WebSocket message")]
    Send {
        #[source]
        source: tokio_tungstenite::tungstenite::Error,
    },

    #[error("failed to receive OBS WebSocket message")]
    Receive {
        #[source]
        source: tokio_tungstenite::tungstenite::Error,
    },

    #[error("OBS WebSocket connection closed")]
    ConnectionClosed,

    #[error("failed to parse OBS WebSocket JSON message")]
    Json {
        #[source]
        source: serde_json::Error,
    },

    #[error("unexpected OBS WebSocket message: {message}")]
    Protocol { message: String },

    #[error("OBS request failed with code {code}: {comment}")]
    RequestFailed { code: u32, comment: String },

    #[error("OBS screenshot response did not include imageData")]
    MissingImageData,

    #[error("OBS screenshot imageData was not a data URL")]
    InvalidImageDataUrl,

    #[error("failed to decode OBS screenshot base64")]
    Base64 {
        #[source]
        source: base64::DecodeError,
    },

    #[error("failed to decode OBS screenshot image")]
    Image {
        #[source]
        source: image::ImageError,
    },

    #[error("invalid ROI rectangle: {message}")]
    InvalidRoi { message: String },
}
