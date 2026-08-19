use std::time::Duration;

use crate::{
    llm::{LlmClient, LlmConfig, LlmError},
    narrative_engine::Emotion,
    obs_vision_adapter::ObsVisionConfig,
    tts::{LocalTtsEngine, TtsConfig, TtsPlayback, TtsPlaybackClass},
};

use super::config::{
    env_llm_api_key, env_llm_base_url, env_llm_model, public_connection_error, ConnectionProvider,
    LauncherConfig, LLM_ENV_HINT, OPENROUTER_BASE_URL, TEST_VOICE_TEXT,
};

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

#[cfg(test)]
mod tests {
    use crate::launcher::active_commentary_runtime_count;

    use super::*;

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
        let before = active_commentary_runtime_count();
        let config = LauncherConfig {
            model: "unit-test-model".into(),
            ..LauncherConfig::default()
        };
        let _ = resolve_llm_config(&config, None);
        assert_eq!(active_commentary_runtime_count(), before);
    }

    #[test]
    fn test_voice_does_not_start_runtime_or_require_llm() {
        let before = active_commentary_runtime_count();
        assert_eq!(TEST_VOICE_TEXT, "这是 AI Commentary 的语音测试。");
        assert_eq!(before, active_commentary_runtime_count());
    }
}
