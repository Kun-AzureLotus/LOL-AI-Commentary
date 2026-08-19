use thiserror::Error;

#[derive(Debug, Error)]
pub enum TtsError {
    #[error("TTS is not available on this platform")]
    UnsupportedPlatform,

    #[error("failed to start Windows SAPI")]
    SpawnFailed,

    #[error("Windows SAPI playback failed")]
    PlaybackFailed,
}
