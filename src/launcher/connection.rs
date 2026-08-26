use std::time::Duration;

use crate::{
    llm::{LlmClient, LlmConfig, LlmError},
    narrative_engine::Emotion,
    obs_vision_adapter::ObsVisionConfig,
    tts::{
        fetch_elevenlabs_voices, ElevenLabsTts, ElevenLabsVoice, LocalTtsEngine, SelectedTtsEngine,
        TtsConfig, TtsError, TtsPlayback, TtsPlaybackClass,
    },
};

use super::config::{
    env_elevenlabs_api_key, env_llm_api_key, env_llm_base_url, env_llm_model,
    public_connection_error, sanitize_error_text, ConnectionProvider, LauncherConfig, TtsProvider,
    UiLanguage, LLM_ENV_HINT, OPENROUTER_BASE_URL, TEST_VOICE_TEXT,
};

pub const ELEVENLABS_API_KEY_NOT_CONFIGURED: &str = "ElevenLabs API Key is not configured.";

pub fn check_startup_requirements(
    config: &LauncherConfig,
    session_api_key: Option<&str>,
) -> Result<LlmConfig, String> {
    let llm = resolve_llm_config(config, session_api_key)?;
    if let Err(error) = ObsVisionConfig::from_env() {
        eprintln!("[OBS Config Error] {error:?}");
    }
    Ok(llm)
}

pub fn create_pipeline_tts(
    config: &LauncherConfig,
    elevenlabs_session_key: Option<&str>,
) -> Result<SelectedTtsEngine, String> {
    create_pipeline_tts_with_keys(config, elevenlabs_session_key, env_elevenlabs_api_key())
}

pub fn create_pipeline_tts_with_keys(
    config: &LauncherConfig,
    elevenlabs_session_key: Option<&str>,
    elevenlabs_env_key: Option<String>,
) -> Result<SelectedTtsEngine, String> {
    match config.tts_provider {
        TtsProvider::Sapi => Ok(SelectedTtsEngine::Sapi(LocalTtsEngine::with_config(
            config.to_tts_config(),
        ))),
        TtsProvider::ElevenLabs => {
            let api_key = elevenlabs_session_key
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .or(elevenlabs_env_key)
                .ok_or_else(|| ELEVENLABS_API_KEY_NOT_CONFIGURED.to_string())?;
            let engine = ElevenLabsTts::new(
                &api_key,
                &config.elevenlabs_voice_id,
                &config.elevenlabs_model,
            )
            .map_err(|error| sanitize_error_text(&error.to_string(), Some(&api_key)))?;
            Ok(SelectedTtsEngine::ElevenLabs(engine))
        }
    }
}

pub fn validate_commentary_start(
    config: &LauncherConfig,
    session_api_key: Option<&str>,
    elevenlabs_session_key: Option<&str>,
    elevenlabs_env_key: Option<String>,
) -> Result<(LlmConfig, SelectedTtsEngine), String> {
    let llm = check_startup_requirements(config, session_api_key)?;
    let tts = create_pipeline_tts_with_keys(config, elevenlabs_session_key, elevenlabs_env_key)?;
    Ok((llm, tts))
}

pub fn resolve_llm_config(
    config: &LauncherConfig,
    session_api_key: Option<&str>,
) -> Result<LlmConfig, String> {
    let base_url = config.base_url.trim();
    let base_url = if base_url.is_empty() {
        match config.provider {
            ConnectionProvider::OpenRouter => OPENROUTER_BASE_URL.to_string(),
            ConnectionProvider::Custom => env_llm_base_url().ok_or_else(|| {
                "Please enter a Base URL for the Custom / OpenAI-Compatible API.".to_string()
            })?,
        }
    } else {
        base_url.to_string()
    };

    let model = config.model.trim();
    let model = if model.is_empty() {
        env_llm_model().ok_or_else(|| LLM_ENV_HINT.to_string())?
    } else {
        model.to_string()
    };

    let api_key = session_api_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(env_llm_api_key)
        .ok_or_else(|| LLM_ENV_HINT.to_string())?;

    Ok(LlmConfig {
        base_url,
        api_key,
        model,
        timeout: Duration::from_secs(30),
    })
}

pub fn test_llm_connection(
    config: &LauncherConfig,
    session_api_key: Option<&str>,
) -> Result<String, String> {
    let mut llm = resolve_llm_config(config, session_api_key)?;
    llm.timeout = Duration::from_secs(12);
    let api_key = llm.api_key.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime.block_on(async {
        let client = LlmClient::new(llm).map_err(|error| {
            public_connection_error(error, Some(&api_key))
        })?;
        match client.complete_prompt("Reply with OK.").await {
            Ok(_) | Err(LlmError::EmptyResponse) => Ok("Connection successful".to_string()),
            Err(error) => Err(public_connection_error(error, Some(&api_key))),
        }
    })
}

pub fn play_test_voice(tts_config: TtsConfig) -> Result<(), String> {
    play_test_voice_text(tts_config, TEST_VOICE_TEXT)
}

pub fn play_test_voice_text(tts_config: TtsConfig, text: &str) -> Result<(), String> {
    let engine = LocalTtsEngine::with_config(tts_config.clone());
    let playback = TtsPlayback::with_config(engine, tts_config);
    playback.enqueue(text, TtsPlaybackClass::Normal, Emotion::Calm);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime
        .block_on(playback.drain())
        .map_err(|error| error.to_string())
}

pub fn resolve_elevenlabs_api_key(session_api_key: Option<&str>) -> Result<String, String> {
    session_api_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(env_elevenlabs_api_key)
        .ok_or_else(|| "Please enter an ElevenLabs API Key.".to_string())
}

pub fn play_elevenlabs_test_voice(
    session_api_key: Option<&str>,
    voice_id: &str,
    model_id: &str,
    text: &str,
    language: UiLanguage,
) -> Result<(), String> {
    let api_key = resolve_elevenlabs_api_key(session_api_key)?;
    let engine = ElevenLabsTts::new(&api_key, voice_id, model_id)
        .map_err(|error| sanitize_error_text(&error.to_string(), Some(&api_key)))?;
    let playback = TtsPlayback::new(engine);
    playback.enqueue(text, TtsPlaybackClass::Normal, Emotion::Calm);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime
        .block_on(playback.drain())
        .map_err(|error| map_elevenlabs_playback_error(error, language, &api_key))
}

pub fn list_elevenlabs_voices(
    session_api_key: Option<&str>,
) -> Result<Vec<ElevenLabsVoice>, String> {
    let api_key = resolve_elevenlabs_api_key(session_api_key)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime
        .block_on(fetch_elevenlabs_voices(&api_key))
        .map_err(|error| sanitize_error_text(&error.to_string(), Some(&api_key)))
}

fn map_elevenlabs_playback_error(error: TtsError, language: UiLanguage, api_key: &str) -> String {
    match error {
        TtsError::QuotaExceeded | TtsError::InvalidVoice => {
            language.elevenlabs_free_voice_error().to_string()
        }
        other => sanitize_error_text(&other.to_string(), Some(api_key)),
    }
}

#[cfg(test)]
mod tests {
    use crate::launcher::active_commentary_runtime_count;
    use crate::tts::{commentary_allows_tts_voice, VoiceLanguageKind};

    use super::*;
    use super::super::config::CommentaryLanguage;

    #[test]
    fn resolve_prefers_session_key_then_does_not_write_it() {
        let config = LauncherConfig {
            provider: ConnectionProvider::OpenRouter,
            base_url: OPENROUTER_BASE_URL.into(),
            model: "test-model".into(),
            ..LauncherConfig::default()
        };
        let resolved = resolve_llm_config(&config, Some("session-secret-key")).unwrap();
        assert_eq!(resolved.base_url, OPENROUTER_BASE_URL);
        assert_eq!(resolved.model, "test-model");
        assert_eq!(resolved.api_key, "session-secret-key");
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("session-secret-key"));
        assert!(!json.to_ascii_lowercase().contains("api_key"));
    }

    #[test]
    fn test_connection_does_not_start_runtime() {
        let config = LauncherConfig {
            model: "unit-test-model".into(),
            ..LauncherConfig::default()
        };
        let _ = resolve_llm_config(&config, None);
        let _ = test_llm_connection;
    }

    #[test]
    fn test_voice_does_not_start_runtime_or_require_llm() {
        let before = active_commentary_runtime_count();
        assert_eq!(TEST_VOICE_TEXT, "这是一段 AI 电竞赛事解说测试语音。");
        assert_eq!(before, active_commentary_runtime_count());
    }

    #[test]
    fn elevenlabs_test_voice_prefers_session_key_and_does_not_start_runtime() {
        let before = active_commentary_runtime_count();
        let key = resolve_elevenlabs_api_key(Some("session-elevenlabs-key")).unwrap();
        assert_eq!(key, "session-elevenlabs-key");
        assert_eq!(before, active_commentary_runtime_count());
        assert_eq!(
            crate::launcher::UiLanguage::Chinese.elevenlabs_free_voice_error(),
            "该音色可能不支持 Free API，请选择可用的默认音色。"
        );
    }

    #[test]
    fn sapi_pipeline_tts_does_not_require_elevenlabs_key() {
        let config = LauncherConfig {
            tts_provider: TtsProvider::Sapi,
            ..LauncherConfig::default()
        };
        let engine = create_pipeline_tts_with_keys(&config, None, None).unwrap();
        assert!(engine.is_sapi());
        assert_eq!(engine.provider_name(), "sapi");
        match engine {
            SelectedTtsEngine::Sapi(local) => {
                assert_eq!(local.config().volume, 80);
            }
            SelectedTtsEngine::ElevenLabs(_) => panic!("SAPI provider must keep LocalTtsEngine"),
        }
    }

    #[test]
    fn elevenlabs_pipeline_tts_uses_settings_voice_and_model() {
        let config = LauncherConfig {
            tts_provider: TtsProvider::ElevenLabs,
            elevenlabs_voice_id: "voice-from-settings".into(),
            elevenlabs_model: "eleven_flash_v2_5".into(),
            ..LauncherConfig::default()
        };
        let engine =
            create_pipeline_tts_with_keys(&config, Some("session-elevenlabs-key"), None).unwrap();
        assert!(engine.is_elevenlabs());
        assert_eq!(engine.provider_name(), "elevenlabs");
        let debug = format!("{engine:?}");
        assert!(debug.contains("voice_id"));
        assert!(debug.contains("voice-from-settings"));
        assert!(debug.contains("eleven_flash_v2_5"));
        assert!(!debug.contains("session-elevenlabs-key"));
        assert!(!debug.contains("xi-api-key"));
    }

    #[test]
    fn elevenlabs_missing_key_blocks_start_without_runtime() {
        let before = active_commentary_runtime_count();
        let config = LauncherConfig {
            model: "unit-test-model".into(),
            tts_provider: TtsProvider::ElevenLabs,
            ..LauncherConfig::default()
        };
        let error = validate_commentary_start(&config, Some("llm-session-key"), None, None)
            .unwrap_err();
        assert_eq!(error, ELEVENLABS_API_KEY_NOT_CONFIGURED);
        assert_eq!(active_commentary_runtime_count(), before);
    }

    #[test]
    fn sapi_start_validation_skips_elevenlabs_key() {
        let config = LauncherConfig {
            model: "unit-test-model".into(),
            tts_provider: TtsProvider::Sapi,
            ..LauncherConfig::default()
        };
        let (_, engine) =
            validate_commentary_start(&config, Some("llm-session-key"), None, None).unwrap();
        assert!(engine.is_sapi());
    }

    #[test]
    fn elevenlabs_env_key_is_not_serialized() {
        let config = LauncherConfig {
            tts_provider: TtsProvider::ElevenLabs,
            elevenlabs_voice_id: "abc".into(),
            elevenlabs_model: "eleven_flash_v2_5".into(),
            ..LauncherConfig::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.to_ascii_lowercase().contains("api_key"));
        assert!(!json.contains("sk_"));
    }

    #[test]
    fn chinese_commentary_with_english_voice_is_allowed() {
        let config = LauncherConfig {
            commentary_language: CommentaryLanguage::SimplifiedChinese,
            tts_provider: TtsProvider::ElevenLabs,
            elevenlabs_voice_id: "sarah-en".into(),
            ..LauncherConfig::default()
        };
        assert!(commentary_allows_tts_voice("zh-CN", VoiceLanguageKind::English));
        let engine =
            create_pipeline_tts_with_keys(&config, Some("session-elevenlabs-key"), None).unwrap();
        assert!(engine.is_elevenlabs());
        assert_eq!(
            config.commentary_language.test_voice_text(),
            "这是一段 AI 电竞赛事解说测试语音。"
        );
    }

    #[test]
    fn english_commentary_with_english_voice_is_allowed() {
        let config = LauncherConfig {
            commentary_language: CommentaryLanguage::English,
            tts_provider: TtsProvider::ElevenLabs,
            elevenlabs_voice_id: "will-en".into(),
            ..LauncherConfig::default()
        };
        assert!(commentary_allows_tts_voice("en", VoiceLanguageKind::English));
        let engine =
            create_pipeline_tts_with_keys(&config, Some("session-elevenlabs-key"), None).unwrap();
        assert!(engine.is_elevenlabs());
        assert_eq!(
            config.commentary_language.test_voice_text(),
            "This is an AI esports commentary test voice."
        );
    }

    #[test]
    fn chinese_commentary_with_chinese_voice_is_allowed() {
        let config = LauncherConfig {
            commentary_language: CommentaryLanguage::SimplifiedChinese,
            tts_provider: TtsProvider::ElevenLabs,
            elevenlabs_voice_id: "cn-female-1".into(),
            ..LauncherConfig::default()
        };
        assert!(commentary_allows_tts_voice("zh-CN", VoiceLanguageKind::Mandarin));
        let engine =
            create_pipeline_tts_with_keys(&config, Some("session-elevenlabs-key"), None).unwrap();
        assert!(engine.is_elevenlabs());
    }

    #[test]
    fn english_commentary_with_chinese_voice_is_allowed() {
        let config = LauncherConfig {
            commentary_language: CommentaryLanguage::English,
            tts_provider: TtsProvider::ElevenLabs,
            elevenlabs_voice_id: "cn-female-1".into(),
            ..LauncherConfig::default()
        };
        assert!(commentary_allows_tts_voice("en", VoiceLanguageKind::Mandarin));
        let engine =
            create_pipeline_tts_with_keys(&config, Some("session-elevenlabs-key"), None).unwrap();
        assert!(engine.is_elevenlabs());
    }
}
