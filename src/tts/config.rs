use crate::narrative_engine::Emotion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtsConfig {
    pub voice_name: Option<String>,
    pub calm_rate: i32,
    pub excited_rate: i32,
    pub epic_rate: i32,
    pub volume: u16,
    pub comma_pause_ms: u32,
    pub sentence_pause_ms: u32,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            voice_name: None,
            calm_rate: -2,
            excited_rate: 1,
            epic_rate: 3,
            volume: 80,
            comma_pause_ms: 140,
            sentence_pause_ms: 260,
        }
    }
}

impl TtsConfig {
    pub fn rate_for_emotion(&self, emotion: Emotion) -> i32 {
        match emotion {
            Emotion::Calm => self.calm_rate,
            Emotion::Excited => self.excited_rate,
            Emotion::Epic => self.epic_rate,
        }
    }

    pub fn clamp_rate(rate: i32) -> i32 {
        rate.clamp(-10, 10)
    }

    pub fn clamp_volume(volume: u16) -> u16 {
        volume.min(100)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceGender {
    Female,
    Male,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledVoice {
    pub name: String,
    pub culture: String,
    pub gender: VoiceGender,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceSelection {
    Named(String),
    SystemDefault,
}

pub fn select_voice(voices: &[InstalledVoice], preferred: Option<&str>) -> VoiceSelection {
    if let Some(preferred) = preferred.map(str::trim).filter(|name| !name.is_empty()) {
        if voices.iter().any(|voice| voice.name == preferred) {
            return VoiceSelection::Named(preferred.to_string());
        }
    }

    if let Some(voice) = voices.iter().find(|voice| {
        voice
            .culture
            .eq_ignore_ascii_case("zh-CN")
    }) {
        return VoiceSelection::Named(voice.name.clone());
    }

    if let Some(voice) = voices.iter().find(|voice| {
        voice
            .culture
            .to_ascii_lowercase()
            .starts_with("zh")
    }) {
        return VoiceSelection::Named(voice.name.clone());
    }

    VoiceSelection::SystemDefault
}

pub fn voices_for_launcher(voices: &[InstalledVoice]) -> Vec<InstalledVoice> {
    let mut zh_cn = Vec::new();
    let mut zh_other = Vec::new();

    for voice in voices {
        let culture = voice.culture.to_ascii_lowercase();
        if culture == "zh-cn" {
            zh_cn.push(voice.clone());
        } else if culture.starts_with("zh") {
            zh_other.push(voice.clone());
        }
    }

    zh_cn.extend(zh_other);
    zh_cn
}

pub fn sort_voices_for_selector(voices: &[InstalledVoice]) -> Vec<InstalledVoice> {
    let mut voices: Vec<InstalledVoice> = voices
        .iter()
        .cloned()
        .map(|mut voice| {
            voice.gender = infer_voice_gender(&voice);
            voice
        })
        .collect();
    voices.sort_by(|left, right| {
        voice_sort_key(left).cmp(&voice_sort_key(right))
    });
    voices
}

pub fn voice_selector_label(voice: &InstalledVoice) -> String {
    let language = if is_chinese_culture(&voice.culture) {
        "中文"
    } else if is_english_culture(&voice.culture) {
        "English"
    } else {
        voice.culture.as_str()
    };
    match infer_voice_gender(voice) {
        VoiceGender::Female => format!("{} · {} · Female", voice.name, language),
        VoiceGender::Male => format!("{} · {} · Male", voice.name, language),
        VoiceGender::Unknown => format!("{} ({})", voice.name, voice.culture),
    }
}

fn is_chinese_culture(culture: &str) -> bool {
    culture.to_ascii_lowercase().starts_with("zh")
}

fn is_english_culture(culture: &str) -> bool {
    culture.to_ascii_lowercase().starts_with("en")
}

fn voice_sort_key(voice: &InstalledVoice) -> (u8, u8, String) {
    let language = if is_chinese_culture(&voice.culture) {
        0
    } else if is_english_culture(&voice.culture) {
        1
    } else {
        2
    };
    let gender = match infer_voice_gender(voice) {
        VoiceGender::Female => 0,
        VoiceGender::Male => 1,
        VoiceGender::Unknown => 2,
    };
    (language, gender, voice.name.to_ascii_lowercase())
}

fn infer_voice_gender(voice: &InstalledVoice) -> VoiceGender {
    if !matches!(voice.gender, VoiceGender::Unknown) {
        return voice.gender;
    }
    let name = voice.name.to_ascii_lowercase();
    const FEMALE: &[&str] = &[
        "xiaoxiao", "huihui", "xiaoyi", "xiaoxuan", "xiaomo", "xiaorui", "yaoyao",
        "zira", "jenny", "aria", "sara", "sonia", "hazel", "susan",
    ];
    const MALE: &[&str] = &[
        "yunxi", "yunyang", "yunjian", "yunye", "yunfeng", "kangkang",
        "david", "guy", "mark", "ryan", "george", "james",
    ];
    if FEMALE.iter().any(|token| name.contains(token)) {
        VoiceGender::Female
    } else if MALE.iter().any(|token| name.contains(token)) {
        VoiceGender::Male
    } else {
        VoiceGender::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice(name: &str, culture: &str) -> InstalledVoice {
        InstalledVoice {
            name: name.to_string(),
            culture: culture.to_string(),
            gender: VoiceGender::Unknown,
        }
    }

    #[test]
    fn prefers_zh_cn_when_no_preferred_voice() {
        let voices = [
            voice("English Voice", "en-US"),
            voice("Chinese Voice", "zh-CN"),
        ];

        assert_eq!(
            select_voice(&voices, None),
            VoiceSelection::Named("Chinese Voice".to_string())
        );
    }

    #[test]
    fn falls_back_to_any_zh_voice() {
        let voices = [
            voice("English Voice", "en-US"),
            voice("Taiwan Voice", "zh-TW"),
        ];

        assert_eq!(
            select_voice(&voices, None),
            VoiceSelection::Named("Taiwan Voice".to_string())
        );
    }

    #[test]
    fn missing_preferred_voice_uses_chinese_then_default() {
        let voices = [voice("English Voice", "en-US")];

        assert_eq!(
            select_voice(&voices, Some("Huihui")),
            VoiceSelection::SystemDefault
        );
    }

    #[test]
    fn prefers_installed_preferred_voice() {
        let voices = [
            voice("Chinese Voice", "zh-CN"),
            voice("Huihui", "zh-CN"),
        ];

        assert_eq!(
            select_voice(&voices, Some("Huihui")),
            VoiceSelection::Named("Huihui".to_string())
        );
    }

    #[test]
    fn calm_excited_epic_use_different_rates() {
        let config = TtsConfig::default();

        assert!(config.rate_for_emotion(Emotion::Calm) < config.rate_for_emotion(Emotion::Excited));
        assert!(config.rate_for_emotion(Emotion::Excited) < config.rate_for_emotion(Emotion::Epic));
    }

    #[test]
    fn launcher_voice_list_puts_zh_cn_before_other_zh() {
        let voices = [
            voice("English Voice", "en-US"),
            voice("Taiwan Voice", "zh-TW"),
            voice("Huihui", "zh-CN"),
        ];

        let filtered = voices_for_launcher(&voices);

        assert_eq!(
            filtered.iter().map(|voice| voice.name.as_str()).collect::<Vec<_>>(),
            vec!["Huihui", "Taiwan Voice"]
        );
    }

    #[test]
    fn launcher_voice_list_is_empty_without_chinese_voices() {
        let voices = [voice("English Voice", "en-US")];

        assert!(voices_for_launcher(&voices).is_empty());
    }
}
