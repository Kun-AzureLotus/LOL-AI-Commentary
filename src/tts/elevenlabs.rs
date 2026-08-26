use std::{
    fs,
    process::{Command, Stdio},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use reqwest::Client;
use serde_json::{json, Value};

use super::{TtsEngine, TtsError, TtsUtterance};

pub const DEFAULT_ELEVENLABS_VOICE_ID: &str = "";
pub const DEFAULT_ELEVENLABS_MODEL: &str = "eleven_flash_v2_5";
pub const DEFAULT_ELEVENLABS_BASE_URL: &str = "https://api.elevenlabs.io";
pub const ELEVENLABS_PCM_SAMPLE_RATE: u32 = 24_000;

#[derive(Clone)]
pub struct ElevenLabsTts {
    http: Client,
    api_key: String,
    voice_id: String,
    model_id: String,
    base_url: String,
}

impl std::fmt::Debug for ElevenLabsTts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElevenLabsTts")
            .field("voice_id", &self.voice_id)
            .field("model_id", &self.model_id)
            .field("base_url", &self.base_url)
            .field("api_key", &"***")
            .finish()
    }
}

impl ElevenLabsTts {
    pub fn new(api_key: impl Into<String>, voice_id: impl Into<String>, model_id: impl Into<String>) -> Result<Self, TtsError> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(TtsError::MissingApiKey);
        }
        let voice_id = nonempty_or(voice_id, DEFAULT_ELEVENLABS_VOICE_ID);
        let model_id = nonempty_or(model_id, DEFAULT_ELEVENLABS_MODEL);
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|_| TtsError::Network)?;
        Ok(Self {
            http,
            api_key,
            voice_id,
            model_id,
            base_url: DEFAULT_ELEVENLABS_BASE_URL.to_string(),
        })
    }

    pub fn prepared_request(&self, text: &str) -> PreparedSpeechRequest {
        PreparedSpeechRequest::new(&self.base_url, &self.voice_id, &self.model_id, text)
    }

    pub async fn synthesize(&self, text: &str) -> Result<Vec<u8>, TtsError> {
        let prepared = self.prepared_request(text);
        let response = self
            .http
            .post(&prepared.url)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("Accept", "application/octet-stream")
            .json(&prepared.body)
            .send()
            .await
            .map_err(map_send_error)?;
        let status = response.status().as_u16();
        let body = response.bytes().await.map_err(map_send_error)?;
        finish_synthesize(Ok((status, body.to_vec())))
    }
}

impl TtsEngine for ElevenLabsTts {
    async fn speak(&self, utterance: &TtsUtterance) -> Result<(), TtsError> {
        let started = Instant::now();
        let pcm = self.synthesize(&utterance.text).await?;
        let wav = pcm16le_mono_to_wav(&pcm, ELEVENLABS_PCM_SAMPLE_RATE)?;
        let result = play_wav_bytes(&wav);
        eprintln!(
            "[TTS Timing] elevenlabs {}ms",
            started.elapsed().as_millis()
        );
        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedSpeechRequest {
    pub url: String,
    pub body: Value,
}

impl PreparedSpeechRequest {
    pub fn new(base_url: &str, voice_id: &str, model_id: &str, text: &str) -> Self {
        let base_url = base_url.trim_end_matches('/');
        Self {
            url: format!(
                "{base_url}/v1/text-to-speech/{voice_id}?output_format=pcm_24000"
            ),
            body: json!({
                "text": text,
                "model_id": model_id,
            }),
        }
    }
}

pub fn finish_synthesize(result: Result<(u16, Vec<u8>), TtsError>) -> Result<Vec<u8>, TtsError> {
    match result {
        Ok((status, body)) => interpret_speech_response(status, &body),
        Err(error) => Err(error),
    }
}

pub fn map_send_error(_error: reqwest::Error) -> TtsError {
    TtsError::Network
}

pub fn interpret_speech_response(status: u16, body: &[u8]) -> Result<Vec<u8>, TtsError> {
    if status == 200 {
        if body.is_empty() {
            return Err(TtsError::EmptyAudio);
        }
        if body.starts_with(b"{")
            || body.starts_with(b"[")
            || body.starts_with(b"ID3")
            || is_mpeg_frame(body)
        {
            return Err(TtsError::Decode);
        }
        return Ok(body.to_vec());
    }

    let code = elevenlabs_error_code(body);
    match (status, code.as_deref()) {
        (401, _) | (_, Some("invalid_api_key")) => Err(TtsError::Unauthorized),
        (429, _) | (_, Some("too_many_requests")) => Err(TtsError::RateLimited),
        (402, _) | (_, Some("quota_exceeded")) => Err(TtsError::QuotaExceeded),
        (404, _) | (_, Some("voice_not_found") | Some("invalid_voice_id")) => {
            Err(TtsError::InvalidVoice)
        }
        (400 | 422, Some(code)) if code.contains("model") => Err(TtsError::InvalidModel),
        (400 | 422, _) => Err(TtsError::InvalidVoice),
        _ => Err(TtsError::Http { status }),
    }
}

pub fn pcm16le_mono_to_wav(pcm: &[u8], sample_rate: u32) -> Result<Vec<u8>, TtsError> {
    if pcm.is_empty() {
        return Err(TtsError::EmptyAudio);
    }
    if pcm.len() % 2 != 0 {
        return Err(TtsError::Decode);
    }
    let data_len = pcm.len() as u32;
    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm);
    Ok(wav)
}

fn elevenlabs_error_code(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;
    value
        .get("detail")
        .and_then(|detail| {
            detail
                .get("status")
                .and_then(Value::as_str)
                .or_else(|| detail.get("type").and_then(Value::as_str))
                .or_else(|| detail.as_str())
        })
        .map(str::to_ascii_lowercase)
}

fn is_mpeg_frame(body: &[u8]) -> bool {
    body.len() >= 2 && body[0] == 0xFF && body[1] & 0xE0 == 0xE0
}

fn nonempty_or(value: impl Into<String>, fallback: &str) -> String {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn play_wav_bytes(wav: &[u8]) -> Result<(), TtsError> {
    #[cfg(not(windows))]
    {
        let _ = wav;
        Err(TtsError::UnsupportedPlatform)
    }

    #[cfg(windows)]
    {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "lol_ai_elevenlabs_{}_{stamp}.wav",
            std::process::id()
        ));
        fs::write(&path, wav).map_err(|_| TtsError::PlaybackFailed)?;
        let escaped = path.to_string_lossy().replace('\'', "''");
        let script = format!("$p = New-Object System.Media.SoundPlayer '{escaped}'; $p.PlaySync()");
        let result = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = fs::remove_file(&path);
        match result {
            Ok(status) if status.success() => Ok(()),
            _ => Err(TtsError::PlaybackFailed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_VOICE_ID: &str = "21m00Tcm4TlvDq8ikWAM";

    #[test]
    fn default_provider_request_uses_specified_voice_and_model() {
        let request = PreparedSpeechRequest::new(
            DEFAULT_ELEVENLABS_BASE_URL,
            SAMPLE_VOICE_ID,
            DEFAULT_ELEVENLABS_MODEL,
            "test commentary",
        );
        assert!(request.url.starts_with("https://api.elevenlabs.io/v1/text-to-speech/"));
        assert!(request.url.contains(SAMPLE_VOICE_ID));
        assert!(request.url.contains("output_format=pcm_24000"));
        assert_eq!(request.body["text"], "test commentary");
        assert_eq!(request.body["model_id"], DEFAULT_ELEVENLABS_MODEL);
        assert!(request.url.contains("/v1/text-to-speech/21m00Tcm4TlvDq8ikWAM"));
    }

    #[test]
    fn ok_response_returns_pcm_bytes() {
        let audio = finish_synthesize(Ok((200, vec![1, 2, 3, 4]))).unwrap();
        assert_eq!(audio, vec![1, 2, 3, 4]);
    }

    #[test]
    fn unauthorized_maps_to_invalid_key() {
        let error = interpret_speech_response(401, br#"{"detail":{"status":"invalid_api_key"}}"#)
            .unwrap_err();
        assert!(matches!(error, TtsError::Unauthorized));
    }

    #[test]
    fn too_many_requests_maps_to_rate_limit() {
        let error = interpret_speech_response(429, b"").unwrap_err();
        assert!(matches!(error, TtsError::RateLimited));
    }

    #[test]
    fn malformed_json_body_on_success_is_decode_error() {
        let error = interpret_speech_response(200, br#"{"detail":"not audio"}"#).unwrap_err();
        assert!(matches!(error, TtsError::Decode));
    }

    #[test]
    fn network_failure_is_preserved() {
        let error = finish_synthesize(Err(TtsError::Network)).unwrap_err();
        assert!(matches!(error, TtsError::Network));
        assert!(!error.to_string().contains("xi-api-key"));
    }

    #[test]
    fn empty_audio_is_rejected() {
        let error = interpret_speech_response(200, b"").unwrap_err();
        assert!(matches!(error, TtsError::EmptyAudio));
    }

    #[test]
    fn quota_and_invalid_voice_are_mapped() {
        let quota = interpret_speech_response(402, br#"{"detail":{"status":"quota_exceeded"}}"#)
            .unwrap_err();
        assert!(matches!(quota, TtsError::QuotaExceeded));
        let voice = interpret_speech_response(404, b"").unwrap_err();
        assert!(matches!(voice, TtsError::InvalidVoice));
    }

    #[test]
    fn wav_wrapper_has_header_and_rejects_odd_pcm() {
        let wav = pcm16le_mono_to_wav(&[0, 1, 2, 3], 24_000).unwrap();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(wav.len(), 48);
        assert!(matches!(
            pcm16le_mono_to_wav(&[0, 1, 2], 24_000),
            Err(TtsError::Decode)
        ));
    }

    #[test]
    fn debug_output_redacts_api_key() {
        let engine = ElevenLabsTts::new(
            "secret-test-key-xyz",
            SAMPLE_VOICE_ID,
            DEFAULT_ELEVENLABS_MODEL,
        )
        .unwrap();
        let debug = format!("{engine:?}");
        assert!(debug.contains("***"));
        assert!(!debug.contains("secret-test-key-xyz"));
    }

    #[test]
    fn missing_key_cannot_construct_engine() {
        let error = ElevenLabsTts::new("  ", SAMPLE_VOICE_ID, DEFAULT_ELEVENLABS_MODEL)
            .unwrap_err();
        assert!(matches!(error, TtsError::MissingApiKey));
    }

    #[test]
    fn engine_builds_official_speech_request() {
        let engine = ElevenLabsTts::new(
            "session-key",
            SAMPLE_VOICE_ID,
            DEFAULT_ELEVENLABS_MODEL,
        )
        .unwrap();
        let request = engine.prepared_request("hello");
        assert_eq!(
            request.url,
            "https://api.elevenlabs.io/v1/text-to-speech/21m00Tcm4TlvDq8ikWAM?output_format=pcm_24000"
        );
        assert_eq!(request.body["text"], "hello");
        assert_eq!(request.body["model_id"], "eleven_flash_v2_5");
    }

    #[test]
    fn invalid_model_is_mapped() {
        let error = interpret_speech_response(400, br#"{"detail":{"status":"invalid_model"}}"#)
            .unwrap_err();
        assert!(matches!(error, TtsError::InvalidModel));
    }

    #[test]
    fn other_http_errors_keep_status() {
        let error = interpret_speech_response(500, b"server").unwrap_err();
        assert!(matches!(error, TtsError::Http { status: 500 }));
        let forbidden = interpret_speech_response(403, b"forbidden").unwrap_err();
        assert!(matches!(forbidden, TtsError::Http { status: 403 }));
    }
}
