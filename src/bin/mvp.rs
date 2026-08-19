use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::Arc;

use lol_ai_commentator::{
    commentary_runtime::{run_commentary_pipeline, CommentaryRuntimeConfig},
    launcher::CommentaryStyle,
    tts::{LocalTtsEngine, TtsConfig, TtsPlayback},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tts_config = TtsConfig::default();
    let tts = TtsPlayback::with_config(
        LocalTtsEngine::with_config(tts_config.clone()),
        tts_config.clone(),
    );
    run_commentary_pipeline(
        CommentaryRuntimeConfig {
            tts: tts_config,
            style: CommentaryStyle::Balanced,
            style_instruction: None,
            output_language: lol_ai_commentator::prompt_builder::PromptOutputLanguage::SimplifiedChinese,
            llm: None,
            obs_hint: Arc::new(AtomicU8::new(0)),
            ai_hint: Arc::new(AtomicU8::new(0)),
        },
        tts,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}
