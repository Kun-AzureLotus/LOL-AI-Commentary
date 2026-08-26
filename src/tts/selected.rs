use super::{ElevenLabsTts, LocalTtsEngine, TtsEngine, TtsError, TtsUtterance};

#[derive(Clone)]
pub enum SelectedTtsEngine {
    Sapi(LocalTtsEngine),
    ElevenLabs(ElevenLabsTts),
}

impl std::fmt::Debug for SelectedTtsEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sapi(_) => f.debug_tuple("Sapi").field(&"LocalTtsEngine").finish(),
            Self::ElevenLabs(engine) => f.debug_tuple("ElevenLabs").field(engine).finish(),
        }
    }
}

impl SelectedTtsEngine {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Sapi(_) => "sapi",
            Self::ElevenLabs(_) => "elevenlabs",
        }
    }

    pub fn is_sapi(&self) -> bool {
        matches!(self, Self::Sapi(_))
    }

    pub fn is_elevenlabs(&self) -> bool {
        matches!(self, Self::ElevenLabs(_))
    }
}

impl TtsEngine for SelectedTtsEngine {
    async fn speak(&self, utterance: &TtsUtterance) -> Result<(), TtsError> {
        match self {
            Self::Sapi(engine) => engine.speak(utterance).await,
            Self::ElevenLabs(engine) => engine.speak(utterance).await,
        }
    }

    fn interrupt(&self) {
        match self {
            Self::Sapi(engine) => engine.interrupt(),
            Self::ElevenLabs(_) => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TtsConnectionHint {
    #[default]
    Unknown,
    Connected,
    Unavailable,
    QuotaExceeded,
}

impl TtsConnectionHint {
    const UNKNOWN: u8 = 0;
    const CONNECTED: u8 = 1;
    const UNAVAILABLE: u8 = 2;
    const QUOTA: u8 = 3;

    pub fn from_u8(value: u8) -> Self {
        match value {
            Self::CONNECTED => Self::Connected,
            Self::UNAVAILABLE => Self::Unavailable,
            Self::QUOTA => Self::QuotaExceeded,
            _ => Self::Unknown,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::Unknown => Self::UNKNOWN,
            Self::Connected => Self::CONNECTED,
            Self::Unavailable => Self::UNAVAILABLE,
            Self::QuotaExceeded => Self::QUOTA,
        }
    }

    pub fn from_error(error: &TtsError) -> Option<Self> {
        match error {
            TtsError::QuotaExceeded => Some(Self::QuotaExceeded),
            TtsError::Unauthorized
            | TtsError::RateLimited
            | TtsError::Network
            | TtsError::Http { .. }
            | TtsError::InvalidVoice
            | TtsError::InvalidModel
            | TtsError::MissingApiKey
            | TtsError::EmptyAudio
            | TtsError::Decode => Some(Self::Unavailable),
            TtsError::UnsupportedPlatform | TtsError::SpawnFailed | TtsError::PlaybackFailed => {
                None
            }
        }
    }

    pub fn status_line(self) -> Option<&'static str> {
        match self {
            Self::Unknown | Self::Connected => None,
            Self::Unavailable => Some("ElevenLabs TTS unavailable"),
            Self::QuotaExceeded => Some("ElevenLabs quota exceeded"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_errors_map_to_nonblocking_status() {
        assert_eq!(
            TtsConnectionHint::from_error(&TtsError::QuotaExceeded),
            Some(TtsConnectionHint::QuotaExceeded)
        );
        assert_eq!(
            TtsConnectionHint::from_error(&TtsError::Unauthorized),
            Some(TtsConnectionHint::Unavailable)
        );
        assert_eq!(
            TtsConnectionHint::from_error(&TtsError::RateLimited),
            Some(TtsConnectionHint::Unavailable)
        );
        assert_eq!(
            TtsConnectionHint::from_error(&TtsError::Network),
            Some(TtsConnectionHint::Unavailable)
        );
        assert_eq!(
            TtsConnectionHint::from_error(&TtsError::Http { status: 403 }),
            Some(TtsConnectionHint::Unavailable)
        );
        assert_eq!(TtsConnectionHint::from_error(&TtsError::PlaybackFailed), None);
        assert_eq!(
            TtsConnectionHint::Unavailable.status_line(),
            Some("ElevenLabs TTS unavailable")
        );
    }
}
