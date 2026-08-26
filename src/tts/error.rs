use thiserror::Error;

#[derive(Debug, Error)]
pub enum TtsError {
    #[error("TTS is not available on this platform")]
    UnsupportedPlatform,

    #[error("failed to start Windows SAPI")]
    SpawnFailed,

    #[error("Windows SAPI playback failed")]
    PlaybackFailed,

    #[error("Please enter an ElevenLabs API Key.")]
    MissingApiKey,

    #[error("ElevenLabs API key is invalid")]
    Unauthorized,

    #[error("ElevenLabs rate limit reached")]
    RateLimited,

    #[error("ElevenLabs quota exceeded")]
    QuotaExceeded,

    #[error("ElevenLabs voice ID is invalid")]
    InvalidVoice,

    #[error("ElevenLabs model is invalid")]
    InvalidModel,

    #[error("ElevenLabs network error")]
    Network,

    #[error("ElevenLabs request failed (HTTP {status})")]
    Http { status: u16 },

    #[error("ElevenLabs returned no audio")]
    EmptyAudio,

    #[error("ElevenLabs audio could not be decoded")]
    Decode,
}
