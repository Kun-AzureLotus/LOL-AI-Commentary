mod config;
mod elevenlabs;
mod elevenlabs_voices;
mod engine;
mod error;
mod local;
mod queue;
mod selected;
mod ssml;

pub use config::{
    select_voice, sort_voices_for_selector, voice_selector_label, voices_for_launcher,
    InstalledVoice, TtsConfig, VoiceGender, VoiceSelection,
};
pub use elevenlabs::{
    ElevenLabsTts, DEFAULT_ELEVENLABS_MODEL, DEFAULT_ELEVENLABS_VOICE_ID,
};
pub use elevenlabs_voices::{
    chinese_voice_count, commentary_allows_tts_voice, english_voice_count, fetch_elevenlabs_voices,
    fetch_elevenlabs_voices_with_stats, preferred_free_voice_id, resolve_picker_voice_id,
    voice_list_stats, voices_for_free_api, ElevenLabsVoice, VoiceLanguageKind, VoiceListStats,
};
pub use engine::{TtsEngine, TtsUtterance};
pub use error::TtsError;
pub use local::{list_installed_voices, LocalTtsEngine};
pub use queue::{EnqueueOutcome, TtsPlayback, TtsPlaybackClass, TtsQueue};
pub use selected::{SelectedTtsEngine, TtsConnectionHint};
pub use ssml::commentary_to_ssml;
