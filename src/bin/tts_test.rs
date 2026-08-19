use std::thread;
use std::time::Duration;

use lol_ai_commentator::narrative_engine::Emotion;
use lol_ai_commentator::tts::{
    select_voice, commentary_to_ssml, InstalledVoice, LocalTtsEngine, TtsConfig, TtsEngine,
    TtsUtterance, VoiceSelection,
};

#[tokio::main]
async fn main() {
    let config = TtsConfig::default();
    let engine = LocalTtsEngine::with_config(config.clone());

    println!("[tts_test] enumerating voice preference: zh-CN / zh-* then system default");
    let demo_voices = [
        InstalledVoice {
            name: "Microsoft Huihui".to_string(),
            culture: "zh-CN".to_string(),
            gender: lol_ai_commentator::tts::VoiceGender::Female,
        },
        InstalledVoice {
            name: "Microsoft David".to_string(),
            culture: "en-US".to_string(),
            gender: lol_ai_commentator::tts::VoiceGender::Male,
        },
    ];
    match select_voice(&demo_voices, config.voice_name.as_deref()) {
        VoiceSelection::Named(name) => println!("[tts_test] preferred installed Chinese voice: {name}"),
        VoiceSelection::SystemDefault => {
            println!("[tts_test] no Chinese voice in demo list, SAPI will use default")
        }
    }

    let samples = [
        (Emotion::Calm, "蓝方开始集中，可能成为焦点。"),
        (Emotion::Excited, "一波击杀打出，节奏已经起来了。"),
        (Emotion::Epic, "大龙被拿下！这是决定性的一击！"),
    ];

    for (emotion, text) in samples {
        let rate = config.rate_for_emotion(emotion);
        println!(
            "[tts_test] {:?} rate={} ssml={}",
            emotion,
            rate,
            commentary_to_ssml(text, config.comma_pause_ms, config.sentence_pause_ms)
        );
        let utterance = TtsUtterance {
            text: text.to_string(),
            rate,
            volume: config.volume,
        };
        if let Err(error) = engine.speak(&utterance).await {
            eprintln!("[TTS Error] {error}");
        }
        thread::sleep(Duration::from_millis(400));
    }

    println!("[tts_test] missing voice fallback uses default SAPI voice");
    let mut missing = config.clone();
    missing.voice_name = Some("Not An Installed Voice".to_string());
    let fallback = LocalTtsEngine::with_config(missing);
    if let Err(error) = fallback
        .speak(&TtsUtterance {
            text: "如果指定语音不存在，就使用系统默认语音。".to_string(),
            rate: config.excited_rate,
            volume: config.volume,
        })
        .await
    {
        eprintln!("[TTS Error] {error}");
    }
}
