use std::collections::BTreeMap;

use reqwest::Client;
use serde_json::Value;

use super::elevenlabs::{map_send_error, DEFAULT_ELEVENLABS_BASE_URL};
use super::TtsError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedLanguage {
    pub language: String,
    pub locale: String,
    pub accent: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElevenLabsVoice {
    pub voice_id: String,
    pub name: String,
    pub category: String,
    pub labels: BTreeMap<String, String>,
    pub description: String,
    pub voice_type: Option<String>,
    pub is_owner: Option<bool>,
    pub sharing_status: Option<String>,
    pub available_for_tiers: Vec<String>,
    pub verified_languages: Vec<VerifiedLanguage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoiceListStats {
    pub total: usize,
    pub default: usize,
    pub personal: usize,
    pub saved: usize,
    pub community: usize,
    pub free_available: usize,
    pub english: usize,
    pub chinese: usize,
    pub other: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceLanguageKind {
    English,
    Mandarin,
    OtherChinese,
    Other,
}

impl ElevenLabsVoice {
    pub fn combo_label(&self) -> String {
        let mut parts = vec![self.name.clone()];
        if let Some(kind) = self.kind_tag() {
            parts.push(kind);
        }
        if let Some(gender) = self.gender_tag() {
            parts.push(gender.to_string());
        }
        parts.join(" · ")
    }

    pub fn is_default(&self) -> bool {
        type_is(self.voice_type.as_deref(), "default") || category_is(&self.category, "premade")
    }

    pub fn is_personal(&self) -> bool {
        type_is(self.voice_type.as_deref(), "personal")
            || category_is(&self.category, "cloned")
            || category_is(&self.category, "generated")
    }

    pub fn is_workspace(&self) -> bool {
        type_is(self.voice_type.as_deref(), "workspace")
    }

    pub fn is_saved(&self) -> bool {
        type_is(self.voice_type.as_deref(), "saved")
    }

    pub fn is_community_library(&self) -> bool {
        if type_contains_community(self.voice_type.as_deref()) {
            return true;
        }
        if self.is_default() || self.is_saved() || self.is_workspace() {
            return false;
        }
        if type_is(self.voice_type.as_deref(), "personal") {
            return false;
        }
        if self.is_owner == Some(true) {
            return false;
        }
        self.sharing_status.is_some()
    }

    pub fn is_free_api_compatible(&self) -> bool {
        if self.voice_id.trim().is_empty() {
            return false;
        }
        if !self.available_on_free_tier() {
            return false;
        }
        if self.is_community_library() {
            return false;
        }
        if self.is_default() || self.is_saved() || self.is_workspace() {
            return true;
        }
        if type_is(self.voice_type.as_deref(), "personal") || self.is_personal() {
            return true;
        }
        if self.is_owner == Some(true) {
            return true;
        }
        false
    }

    fn available_on_free_tier(&self) -> bool {
        if self.available_for_tiers.is_empty() {
            return true;
        }
        self.available_for_tiers.iter().any(|tier| {
            let tier = tier.trim().to_ascii_lowercase();
            tier == "free" || tier == "free_tier"
        })
    }

    pub fn language_kind(&self) -> VoiceLanguageKind {
        if self.primary_is_english() {
            return VoiceLanguageKind::English;
        }
        if self.is_cantonese() {
            return VoiceLanguageKind::OtherChinese;
        }
        if self.is_mandarin() {
            return VoiceLanguageKind::Mandarin;
        }
        if self.is_chinese_language() {
            return VoiceLanguageKind::OtherChinese;
        }
        if self.is_english_language() {
            return VoiceLanguageKind::English;
        }
        VoiceLanguageKind::Other
    }

    pub fn is_english_native(&self) -> bool {
        self.language_kind() == VoiceLanguageKind::English
    }

    pub fn is_chinese_candidate(&self) -> bool {
        matches!(
            self.language_kind(),
            VoiceLanguageKind::Mandarin | VoiceLanguageKind::OtherChinese
        )
    }

    pub fn is_mandarin(&self) -> bool {
        if self.is_cantonese() || self.primary_is_english() {
            return false;
        }
        let language = self.label_ci("language");
        let accent = self.label_ci("accent");
        language_is_mandarin(language.as_deref())
            || accent_is_mandarin(accent.as_deref())
            || description_has_mandarin(&self.description)
    }

    fn is_chinese_language(&self) -> bool {
        if self.primary_is_english() {
            return false;
        }
        language_is_chinese(self.label_ci("language").as_deref())
            || language_is_chinese(self.label_ci("locale").as_deref())
            || description_has_chinese(&self.description)
    }

    fn is_english_language(&self) -> bool {
        language_is_english(self.label_ci("language").as_deref())
            || description_has_english(&self.description)
    }

    fn primary_is_english(&self) -> bool {
        language_is_english(self.label_ci("language").as_deref())
            && !language_is_chinese(self.label_ci("language").as_deref())
            && !language_is_mandarin(self.label_ci("language").as_deref())
    }

    fn is_cantonese(&self) -> bool {
        let blob = self.metadata_blob();
        blob.contains("cantonese")
            || blob.contains("yue")
            || blob.contains("粤语")
            || blob.contains("粵語")
            || blob.contains("广东话")
            || blob.contains("廣東話")
    }

    pub fn gender_tag(&self) -> Option<&'static str> {
        match self.label_ci("gender").as_deref() {
            Some(value) if value.contains("female") || value.contains("woman") => Some("Female"),
            Some(value) if value.contains("male") || value.contains("man") => Some("Male"),
            _ => None,
        }
    }

    fn kind_tag(&self) -> Option<String> {
        match self.language_kind() {
            VoiceLanguageKind::English => Some("English".into()),
            VoiceLanguageKind::Mandarin | VoiceLanguageKind::OtherChinese => Some("Chinese".into()),
            VoiceLanguageKind::Other => self.label_ci("language").and_then(|language| {
                let language = language.trim();
                if language.is_empty() {
                    None
                } else {
                    Some(title_case_language(language))
                }
            }),
        }
    }

    pub fn is_narration_suitable(&self) -> bool {
        if self.is_character_or_asmr() {
            return false;
        }
        let blob = self.metadata_blob();
        blob.contains("narrat")
            || blob.contains("informative")
            || blob.contains("entertainment")
            || blob.contains("news")
            || blob.contains("commentary")
            || blob.contains("conversational")
            || blob.contains("professional")
            || blob.contains("natural")
            || blob.contains("calm")
            || blob.contains("clear")
    }

    pub fn is_character_or_asmr(&self) -> bool {
        let blob = self.metadata_blob();
        blob.contains("asmr")
            || blob.contains("whisper")
            || blob.contains("anime")
            || blob.contains("cartoon")
            || blob.contains("character")
            || blob.contains("theatrical")
            || blob.contains("overdramatic")
            || blob.contains("dialect")
    }

    fn label_ci(&self, key: &str) -> Option<String> {
        self.labels
            .iter()
            .find(|(item, _)| item.eq_ignore_ascii_case(key))
            .map(|(_, value)| value.to_ascii_lowercase())
    }

    fn metadata_blob(&self) -> String {
        let mut parts = Vec::new();
        for (key, value) in &self.labels {
            parts.push(key.to_ascii_lowercase());
            parts.push(value.to_ascii_lowercase());
        }
        parts.push(self.description.to_ascii_lowercase());
        if let Some(voice_type) = &self.voice_type {
            parts.push(voice_type.to_ascii_lowercase());
        }
        parts.push(self.category.to_ascii_lowercase());
        for item in &self.verified_languages {
            parts.push(item.language.to_ascii_lowercase());
            parts.push(item.locale.to_ascii_lowercase());
            parts.push(item.accent.to_ascii_lowercase());
        }
        parts.join(" ")
    }

    fn sort_rank(&self) -> (u8, u8, u8, u8) {
        let group = match self.language_kind() {
            VoiceLanguageKind::English => 0,
            VoiceLanguageKind::Mandarin => 1,
            VoiceLanguageKind::OtherChinese => 2,
            VoiceLanguageKind::Other => 3,
        };
        let style = if self.is_character_or_asmr() {
            2
        } else if self.is_narration_suitable() {
            0
        } else {
            1
        };
        let gender = match self.gender_tag() {
            Some("Female") => 0,
            Some("Male") => 1,
            _ => 2,
        };
        let source = if self.is_default() {
            0
        } else if self.is_saved() {
            1
        } else if type_is(self.voice_type.as_deref(), "personal") || self.is_workspace() {
            2
        } else {
            3
        };
        (group, style, gender, source)
    }
}

pub fn voices_url(base_url: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    format!("{base_url}/v1/voices?show_legacy=true")
}

pub fn voices_url_for_type(base_url: &str, voice_type: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    format!("{base_url}/v1/voices?voice_type={voice_type}&show_legacy=true")
}

pub fn merge_voice_lists(lists: impl IntoIterator<Item = Vec<ElevenLabsVoice>>) -> Vec<ElevenLabsVoice> {
    let mut by_id = BTreeMap::new();
    for list in lists {
        for voice in list {
            by_id.entry(voice.voice_id.clone()).or_insert(voice);
        }
    }
    by_id.into_values().collect()
}

pub fn interpret_voices_response(status: u16, body: &[u8]) -> Result<Vec<ElevenLabsVoice>, TtsError> {
    if status != 200 {
        return Err(map_voices_http_error(status, body));
    }
    let value: Value = serde_json::from_slice(body).map_err(|_| TtsError::Decode)?;
    let Some(items) = value.get("voices").and_then(Value::as_array) else {
        return Err(TtsError::Decode);
    };
    let mut voices = Vec::new();
    for item in items {
        let voice_id = json_string(item.get("voice_id"));
        if voice_id.is_empty() {
            continue;
        }
        voices.push(ElevenLabsVoice {
            name: json_string(item.get("name")),
            category: json_string(item.get("category")),
            labels: json_string_map(item.get("labels")),
            description: json_string(item.get("description")),
            voice_type: optional_json_string(item.get("voice_type")),
            is_owner: item.get("is_owner").and_then(Value::as_bool),
            sharing_status: sharing_status(item.get("sharing")),
            available_for_tiers: json_string_list(item.get("available_for_tiers")),
            verified_languages: json_verified_languages(item.get("verified_languages")),
            voice_id,
        });
    }
    Ok(voices)
}

pub fn voices_for_free_api(voices: Vec<ElevenLabsVoice>) -> Vec<ElevenLabsVoice> {
    let mut voices: Vec<ElevenLabsVoice> = voices
        .into_iter()
        .filter(ElevenLabsVoice::is_free_api_compatible)
        .collect();
    voices.sort_by(|left, right| {
        left.sort_rank()
            .cmp(&right.sort_rank())
            .then_with(|| left.name.to_ascii_lowercase().cmp(&right.name.to_ascii_lowercase()))
    });
    voices
}

pub fn preferred_free_voice_id(voices: &[ElevenLabsVoice]) -> Option<String> {
    voices
        .iter()
        .find(|voice| {
            voice.is_english_native()
                && voice.is_narration_suitable()
                && voice.gender_tag() == Some("Female")
        })
        .or_else(|| {
            voices.iter().find(|voice| {
                voice.is_english_native()
                    && voice.is_narration_suitable()
                    && voice.gender_tag() == Some("Male")
            })
        })
        .or_else(|| {
            voices
                .iter()
                .find(|voice| voice.is_english_native() && voice.is_narration_suitable())
        })
        .or_else(|| {
            voices.iter().find(|voice| {
                voice.is_english_native() && voice.gender_tag() == Some("Female")
            })
        })
        .or_else(|| {
            voices
                .iter()
                .find(|voice| voice.is_english_native() && voice.gender_tag() == Some("Male"))
        })
        .or_else(|| voices.iter().find(|voice| voice.is_english_native()))
        .or_else(|| {
            voices
                .iter()
                .find(|voice| voice.is_chinese_candidate() && !voice.is_character_or_asmr())
        })
        .or_else(|| voices.iter().find(|voice| voice.is_default()))
        .or_else(|| voices.first())
        .map(|voice| voice.voice_id.clone())
}

pub fn resolve_picker_voice_id(saved: &str, voices: &[ElevenLabsVoice]) -> Option<String> {
    let saved = saved.trim();
    if !saved.is_empty() && voices.iter().any(|voice| voice.voice_id == saved) {
        return Some(saved.to_string());
    }
    preferred_free_voice_id(voices)
}

pub fn chinese_voice_count(voices: &[ElevenLabsVoice]) -> usize {
    voices
        .iter()
        .filter(|voice| voice.is_chinese_candidate())
        .count()
}

pub fn english_voice_count(voices: &[ElevenLabsVoice]) -> usize {
    voices
        .iter()
        .filter(|voice| voice.is_english_native())
        .count()
}

pub fn other_voice_count(voices: &[ElevenLabsVoice]) -> usize {
    voices
        .iter()
        .filter(|voice| voice.language_kind() == VoiceLanguageKind::Other)
        .count()
}

pub fn voice_list_stats(raw: &[ElevenLabsVoice], filtered: &[ElevenLabsVoice]) -> VoiceListStats {
    VoiceListStats {
        total: raw.len(),
        default: raw.iter().filter(|voice| voice.is_default()).count(),
        personal: raw
            .iter()
            .filter(|voice| type_is(voice.voice_type.as_deref(), "personal"))
            .count(),
        saved: raw.iter().filter(|voice| voice.is_saved()).count(),
        community: raw
            .iter()
            .filter(|voice| voice.is_community_library())
            .count(),
        free_available: filtered.len(),
        english: english_voice_count(filtered),
        chinese: chinese_voice_count(filtered),
        other: other_voice_count(filtered),
    }
}

pub fn log_voice_list_stats(stats: VoiceListStats) {
    #[cfg(debug_assertions)]
    {
        eprintln!(
            "[ElevenLabs] Total API voices: {}\nDefault: {}\nPersonal: {}\nSaved: {}\nCommunity: {}\nFree candidates: {}\nEnglish: {}\nChinese: {}\nOther: {}",
            stats.total,
            stats.default,
            stats.personal,
            stats.saved,
            stats.community,
            stats.free_available,
            stats.english,
            stats.chinese,
            stats.other
        );
    }
    let _ = stats;
}

/// Commentary language and TTS voice language are independent.
/// Chinese commentary may use an English-native ElevenLabs voice.
pub fn commentary_allows_tts_voice(
    _commentary_language: &str,
    _voice_kind: VoiceLanguageKind,
) -> bool {
    true
}

pub fn finish_voices(result: Result<(u16, Vec<u8>), TtsError>) -> Result<Vec<ElevenLabsVoice>, TtsError> {
    match result {
        Ok((status, body)) => interpret_voices_response(status, &body),
        Err(error) => Err(error),
    }
}

pub async fn fetch_elevenlabs_voices(api_key: &str) -> Result<Vec<ElevenLabsVoice>, TtsError> {
    fetch_elevenlabs_voices_with_stats(api_key)
        .await
        .map(|(voices, _)| voices)
}

pub async fn fetch_elevenlabs_voices_with_stats(
    api_key: &str,
) -> Result<(Vec<ElevenLabsVoice>, VoiceListStats), TtsError> {
    if api_key.trim().is_empty() {
        return Err(TtsError::MissingApiKey);
    }
    let http = Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|_| TtsError::Network)?;
    let all = request_voice_list(&http, api_key, &voices_url(DEFAULT_ELEVENLABS_BASE_URL)).await?;
    let defaults = request_voice_list_optional(
        &http,
        api_key,
        &voices_url_for_type(DEFAULT_ELEVENLABS_BASE_URL, "default"),
    )
    .await;
    let saved = request_voice_list_optional(
        &http,
        api_key,
        &voices_url_for_type(DEFAULT_ELEVENLABS_BASE_URL, "saved"),
    )
    .await;
    let raw = merge_voice_lists([all, defaults, saved]);
    let filtered = voices_for_free_api(raw.clone());
    let stats = voice_list_stats(&raw, &filtered);
    log_voice_list_stats(stats);
    Ok((filtered, stats))
}

async fn request_voice_list_optional(http: &Client, api_key: &str, url: &str) -> Vec<ElevenLabsVoice> {
    request_voice_list(http, api_key, url)
        .await
        .unwrap_or_default()
}

async fn request_voice_list(
    http: &Client,
    api_key: &str,
    url: &str,
) -> Result<Vec<ElevenLabsVoice>, TtsError> {
    let response = http
        .get(url)
        .header("xi-api-key", api_key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(map_send_error)?;
    let status = response.status().as_u16();
    let body = response.bytes().await.map_err(map_send_error)?;
    finish_voices(Ok((status, body.to_vec())))
}

fn map_voices_http_error(status: u16, body: &[u8]) -> TtsError {
    let code = error_code(body);
    match (status, code.as_deref()) {
        (401, _) | (_, Some("invalid_api_key")) => TtsError::Unauthorized,
        (429, _) | (_, Some("too_many_requests")) => TtsError::RateLimited,
        (402, _) | (_, Some("quota_exceeded")) => TtsError::QuotaExceeded,
        _ => TtsError::Http { status },
    }
}

fn error_code(body: &[u8]) -> Option<String> {
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

fn json_string(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn optional_json_string(value: Option<&Value>) -> Option<String> {
    let text = json_string(value);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn json_string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    let Some(object) = value.and_then(Value::as_object) else {
        return BTreeMap::new();
    };
    object
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|text| (key.clone(), text.to_string())))
        .collect()
}

fn sharing_status(value: Option<&Value>) -> Option<String> {
    let sharing = value?;
    if sharing.is_null() {
        return None;
    }
    sharing
        .get("status")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| {
            sharing
                .get("original_voice_id")
                .and_then(Value::as_str)
                .map(|_| "copied".to_string())
        })
}

fn json_string_list(value: Option<&Value>) -> Vec<String> {
    let Some(items) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn json_verified_languages(value: Option<&Value>) -> Vec<VerifiedLanguage> {
    let Some(items) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            if !item.is_object() {
                return None;
            }
            let language = json_string(item.get("language"));
            let locale = json_string(item.get("locale"));
            let accent = json_string(item.get("accent"));
            if language.is_empty() && locale.is_empty() && accent.is_empty() {
                return None;
            }
            Some(VerifiedLanguage {
                language,
                locale,
                accent,
            })
        })
        .collect()
}

fn language_is_chinese(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let value = normalize_locale(value);
    matches!(
        value.as_str(),
        "zh" | "zh-cn" | "zh-tw" | "zh-hk" | "cmn" | "chinese" | "中文"
    ) || value.starts_with("zh-")
        || value.contains("chinese")
}

fn language_is_mandarin(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let value = normalize_locale(value);
    value.contains("mandarin")
        || value == "cmn"
        || value.starts_with("cmn-")
        || value == "zh"
        || value == "zh-cn"
        || value == "zh-tw"
        || value.contains("普通话")
        || value.contains("普通話")
}

fn language_is_english(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let value = normalize_locale(value);
    value == "en"
        || value.starts_with("en-")
        || value.contains("english")
        || value == "american"
        || value == "british"
}

fn accent_is_mandarin(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    value.contains("mandarin") || value.contains("普通话") || value.contains("普通話")
}

fn description_has_mandarin(description: &str) -> bool {
    let text = description.to_ascii_lowercase();
    text.contains("mandarin chinese")
        || text.contains("chinese mandarin")
        || text.contains("mandarin")
        || description.contains("普通话")
        || description.contains("普通話")
}

fn description_has_chinese(description: &str) -> bool {
    let text = description.to_ascii_lowercase();
    text.contains("chinese") || description.contains("中文")
}

fn description_has_english(description: &str) -> bool {
    description.to_ascii_lowercase().contains("english")
}

fn normalize_locale(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('_', "-")
}

fn type_is(value: Option<&str>, expected: &str) -> bool {
    value
        .map(str::to_ascii_lowercase)
        .is_some_and(|value| value == expected)
}

fn type_contains_community(value: Option<&str>) -> bool {
    value
        .map(str::to_ascii_lowercase)
        .is_some_and(|value| value == "community" || value.contains("community"))
}

fn category_is(value: &str, expected: &str) -> bool {
    value.eq_ignore_ascii_case(expected)
}

fn title_case_language(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_list_json() -> Vec<u8> {
        br#"{
            "voices": [
                {
                    "voice_id": "libvoice",
                    "name": "Library Copy",
                    "category": "generated",
                    "labels": {"accent": "american", "language": "chinese"},
                    "description": "from library",
                    "voice_type": "community",
                    "is_owner": false,
                    "sharing": {"status": "copied", "original_voice_id": "abc"}
                },
                {
                    "voice_id": "paid-only",
                    "name": "Studio Mandarin",
                    "category": "premade",
                    "labels": {"language": "mandarin", "gender": "female", "accent": "standard"},
                    "description": "Mandarin Chinese narration",
                    "voice_type": "default",
                    "available_for_tiers": ["creator", "pro"]
                },
                {
                    "voice_id": "21m00Tcm4TlvDq8ikWAM",
                    "name": "Rachel",
                    "category": "premade",
                    "labels": {"gender": "female", "language": "english", "use case": "narration"},
                    "description": "calm default",
                    "voice_type": "default",
                    "available_for_tiers": ["free", "starter"]
                },
                {
                    "voice_id": "en-male-1",
                    "name": "Adam",
                    "category": "premade",
                    "labels": {"gender": "male", "language": "en-US", "accent": "american", "use case": "narration"},
                    "description": "natural professional",
                    "voice_type": "default"
                },
                {
                    "voice_id": "en-gb-female",
                    "name": "Lily",
                    "category": "premade",
                    "labels": {"gender": "female", "language": "en-GB", "accent": "british"},
                    "description": "clear conversational english",
                    "voice_type": "default"
                },
                {
                    "voice_id": "cn-female-1",
                    "name": "Lin",
                    "category": "premade",
                    "labels": {"language": "zh-cn", "gender": "female", "accent": "standard"},
                    "description": "Mandarin Chinese news style",
                    "voice_type": "default"
                },
                {
                    "voice_id": "cn-male-1",
                    "name": "Hao",
                    "category": "cloned",
                    "labels": {"language": "mandarin", "gender": "male"},
                    "description": "clear Mandarin Chinese",
                    "voice_type": "personal",
                    "is_owner": true
                },
                {
                    "voice_id": "name-only-taiwan",
                    "name": "Taiwan Girl",
                    "category": "premade",
                    "labels": {"language": "english", "accent": "american", "gender": "female"},
                    "description": "young american english",
                    "voice_type": "default"
                },
                {
                    "voice_id": "cloned-1",
                    "name": "My Clone",
                    "category": "cloned",
                    "labels": {},
                    "description": "personal clone",
                    "voice_type": "personal",
                    "is_owner": true
                },
                {
                    "voice_id": "saved-1",
                    "name": "Saved Narrator",
                    "category": "cloned",
                    "labels": {"language": "english", "gender": "male", "use case": "narration"},
                    "description": "saved personal voice",
                    "voice_type": "saved",
                    "is_owner": true
                },
                {
                    "voice_id": "personal-paid",
                    "name": "Studio Clone",
                    "category": "cloned",
                    "labels": {"language": "english", "gender": "female"},
                    "description": "personal but paid only",
                    "voice_type": "personal",
                    "is_owner": true,
                    "available_for_tiers": ["creator", "pro"]
                },
                {
                    "voice_id": "es-female-1",
                    "name": "Valentina",
                    "category": "premade",
                    "labels": {"language": "spanish", "gender": "female", "accent": "latin american"},
                    "description": "clear Spanish narration",
                    "voice_type": "default"
                },
                {
                    "voice_id": "ws-1",
                    "name": "Workspace Host",
                    "category": "cloned",
                    "labels": {"language": "english", "gender": "female", "use case": "narration"},
                    "description": "workspace voice",
                    "voice_type": "workspace",
                    "available_for_tiers": ["free"]
                }
            ]
        }"#
        .to_vec()
    }

    fn real_api_shape_json() -> Vec<u8> {
        br#"{
            "voices": [
                {
                    "voice_id": "9lHjugDhwqoxA5MhX0az",
                    "name": "Anna Su - Casual, Conversational, Authentic",
                    "category": "professional",
                    "labels": {"accent": "taiwan mandarin", "gender": "female", "language": "zh"},
                    "description": "A warm conversational voice",
                    "voice_type": null,
                    "is_owner": false,
                    "sharing": {"status": "copied", "original_voice_id": "orig-anna"},
                    "available_for_tiers": [],
                    "verified_languages": [
                        {"language": "zh", "locale": "cmn-TW", "accent": "taiwan mandarin"}
                    ]
                },
                {
                    "voice_id": "sarah-en",
                    "name": "Sarah",
                    "category": "premade",
                    "labels": {"accent": "american", "gender": "female", "language": "en", "use case": "narration"},
                    "description": "A young adult woman",
                    "voice_type": "default",
                    "available_for_tiers": [],
                    "verified_languages": [
                        {"language": "en", "locale": "en-US", "accent": "american"},
                        {"language": "zh", "locale": "cmn-CN", "accent": "standard"}
                    ]
                },
                {
                    "voice_id": "will-en",
                    "name": "Will",
                    "category": "premade",
                    "labels": {"accent": "american", "gender": "male", "language": "en"},
                    "description": "conversational",
                    "voice_type": "default",
                    "available_for_tiers": []
                }
            ]
        }"#
        .to_vec()
    }

    #[test]
    fn voices_url_uses_official_endpoint() {
        assert_eq!(
            voices_url("https://api.elevenlabs.io"),
            "https://api.elevenlabs.io/v1/voices?show_legacy=true"
        );
        assert_eq!(
            voices_url_for_type("https://api.elevenlabs.io", "default"),
            "https://api.elevenlabs.io/v1/voices?voice_type=default&show_legacy=true"
        );
        assert_eq!(
            voices_url_for_type("https://api.elevenlabs.io", "saved"),
            "https://api.elevenlabs.io/v1/voices?voice_type=saved&show_legacy=true"
        );
    }

    #[test]
    fn merge_keeps_default_voices_with_personal() {
        let personal_only = interpret_voices_response(
            200,
            br#"{"voices":[{"voice_id":"cloned-1","name":"My Clone","category":"cloned","voice_type":"personal","is_owner":true,"labels":{},"description":"personal clone"}]}"#,
        )
        .unwrap();
        let defaults = interpret_voices_response(
            200,
            br#"{"voices":[{"voice_id":"21m00Tcm4TlvDq8ikWAM","name":"Rachel","category":"premade","voice_type":"default","labels":{"language":"english","gender":"female"},"description":"calm default"}]}"#,
        )
        .unwrap();
        let merged = voices_for_free_api(merge_voice_lists([personal_only, defaults]));
        let ids: Vec<&str> = merged.iter().map(|voice| voice.voice_id.as_str()).collect();
        assert!(ids.contains(&"cloned-1"));
        assert!(ids.contains(&"21m00Tcm4TlvDq8ikWAM"));
    }

    #[test]
    fn default_voice_enters_free_candidates() {
        let voices = voices_for_free_api(interpret_voices_response(200, &sample_list_json()).unwrap());
        assert!(voices.iter().any(|voice| voice.voice_id == "21m00Tcm4TlvDq8ikWAM" && voice.is_default()));
        assert!(voices.iter().any(|voice| voice.voice_id == "en-male-1" && voice.is_default()));
        assert!(voices.iter().any(|voice| voice.voice_id == "cn-female-1" && voice.is_default()));
    }

    #[test]
    fn personal_free_voice_enters_free_candidates() {
        let voices = voices_for_free_api(interpret_voices_response(200, &sample_list_json()).unwrap());
        let clone = voices.iter().find(|voice| voice.voice_id == "cloned-1").unwrap();
        assert!(clone.is_personal());
        assert!(clone.is_free_api_compatible());
        let hao = voices.iter().find(|voice| voice.voice_id == "cn-male-1").unwrap();
        assert!(type_is(hao.voice_type.as_deref(), "personal"));
    }

    #[test]
    fn community_voice_is_excluded_from_free_api() {
        let voices = voices_for_free_api(interpret_voices_response(200, &sample_list_json()).unwrap());
        assert!(!voices.iter().any(|voice| voice.voice_id == "libvoice"));
        let raw = interpret_voices_response(200, &sample_list_json()).unwrap();
        let community = raw.iter().find(|voice| voice.voice_id == "libvoice").unwrap();
        assert!(community.is_community_library());
        assert!(!community.is_free_api_compatible());
    }

    #[test]
    fn paid_only_voice_is_excluded() {
        let voices = voices_for_free_api(interpret_voices_response(200, &sample_list_json()).unwrap());
        assert!(!voices.iter().any(|voice| voice.voice_id == "paid-only"));
        assert!(!voices.iter().any(|voice| voice.voice_id == "personal-paid"));
    }

    #[test]
    fn saved_voice_is_kept() {
        let voices = voices_for_free_api(interpret_voices_response(200, &sample_list_json()).unwrap());
        let saved = voices.iter().find(|voice| voice.voice_id == "saved-1").unwrap();
        assert!(saved.is_saved());
        assert!(saved.is_free_api_compatible());
    }

    #[test]
    fn other_language_free_voice_is_kept() {
        let voices = voices_for_free_api(interpret_voices_response(200, &sample_list_json()).unwrap());
        let spanish = voices.iter().find(|voice| voice.voice_id == "es-female-1").unwrap();
        assert_eq!(spanish.language_kind(), VoiceLanguageKind::Other);
        assert_eq!(spanish.combo_label(), "Valentina · Spanish · Female");
    }

    #[test]
    fn voices_success_filters_community_and_paid_only() {
        let voices = voices_for_free_api(interpret_voices_response(200, &sample_list_json()).unwrap());
        let ids: Vec<&str> = voices.iter().map(|voice| voice.voice_id.as_str()).collect();
        assert!(!ids.contains(&"libvoice"));
        assert!(!ids.contains(&"paid-only"));
        assert!(!ids.contains(&"personal-paid"));
        assert!(ids.contains(&"21m00Tcm4TlvDq8ikWAM"));
        assert!(ids.contains(&"en-male-1"));
        assert!(ids.contains(&"cn-female-1"));
        assert!(ids.contains(&"cloned-1"));
        assert!(ids.contains(&"saved-1"));
        assert!(ids.contains(&"es-female-1"));
        assert!(ids.contains(&"ws-1"));
        assert!(voices[0].is_english_native());
        assert_eq!(voices[0].gender_tag(), Some("Female"));
    }

    #[test]
    fn english_native_voices_are_preferred_default() {
        let voices = voices_for_free_api(interpret_voices_response(200, &sample_list_json()).unwrap());
        assert!(english_voice_count(&voices) >= 3);
        assert_eq!(chinese_voice_count(&voices), 2);
        assert!(other_voice_count(&voices) >= 1);
        let preferred = preferred_free_voice_id(&voices).unwrap();
        let preferred_voice = voices.iter().find(|voice| voice.voice_id == preferred).unwrap();
        assert!(preferred_voice.is_english_native());
        assert_eq!(preferred_voice.gender_tag(), Some("Female"));
        assert!(preferred_voice.is_narration_suitable());
        let rachel = voices
            .iter()
            .find(|voice| voice.voice_id == "21m00Tcm4TlvDq8ikWAM")
            .unwrap();
        assert!(rachel.is_english_native());
        assert_eq!(rachel.combo_label(), "Rachel · English · Female");
        let adam = voices.iter().find(|voice| voice.voice_id == "en-male-1").unwrap();
        assert_eq!(adam.combo_label(), "Adam · English · Male");
        let lily = voices.iter().find(|voice| voice.voice_id == "en-gb-female").unwrap();
        assert!(lily.is_english_native());
        let lin = voices.iter().find(|voice| voice.voice_id == "cn-female-1").unwrap();
        assert!(lin.is_chinese_candidate());
        assert_eq!(lin.combo_label(), "Lin · Chinese · Female");
        assert!(!rachel.combo_label().contains("台湾普通话"));
        assert!(!lin.combo_label().contains("台湾普通话"));
    }

    #[test]
    fn previously_saved_voice_is_kept_when_still_available() {
        let voices = voices_for_free_api(interpret_voices_response(200, &sample_list_json()).unwrap());
        assert_eq!(
            resolve_picker_voice_id("saved-1", &voices).as_deref(),
            Some("saved-1")
        );
        assert_eq!(
            resolve_picker_voice_id("cn-female-1", &voices).as_deref(),
            Some("cn-female-1")
        );
        let fallback = resolve_picker_voice_id("missing-id", &voices).unwrap();
        let fallback_voice = voices.iter().find(|voice| voice.voice_id == fallback).unwrap();
        assert!(fallback_voice.is_english_native());
        assert_eq!(fallback_voice.gender_tag(), Some("Female"));
    }

    #[test]
    fn voice_library_professional_copy_is_excluded() {
        let raw = interpret_voices_response(200, &real_api_shape_json()).unwrap();
        let voices = voices_for_free_api(raw.clone());
        let ids: Vec<&str> = voices.iter().map(|voice| voice.voice_id.as_str()).collect();
        assert!(!ids.contains(&"9lHjugDhwqoxA5MhX0az"));
        assert!(ids.contains(&"sarah-en"));
        assert!(ids.contains(&"will-en"));
        let sarah = voices.iter().find(|voice| voice.voice_id == "sarah-en").unwrap();
        assert!(sarah.is_english_native());
        assert!(!sarah.is_chinese_candidate());
        assert_eq!(preferred_free_voice_id(&voices).as_deref(), Some("sarah-en"));
        let stats = voice_list_stats(&raw, &voices);
        assert_eq!(stats.total, 3);
        assert_eq!(stats.default, 2);
        assert_eq!(stats.community, 1);
        assert_eq!(stats.free_available, 2);
        assert_eq!(stats.chinese, 0);
        assert_eq!(stats.english, 2);
    }

    #[test]
    fn chinese_commentary_may_use_english_native_voice() {
        assert!(commentary_allows_tts_voice("zh-CN", VoiceLanguageKind::English));
        assert!(commentary_allows_tts_voice("zh-TW", VoiceLanguageKind::English));
        assert!(commentary_allows_tts_voice("en", VoiceLanguageKind::English));
        assert!(commentary_allows_tts_voice("zh-CN", VoiceLanguageKind::Mandarin));
        assert!(commentary_allows_tts_voice("en", VoiceLanguageKind::Mandarin));
    }

    #[test]
    fn empty_list_is_ok() {
        let voices = interpret_voices_response(200, br#"{"voices":[]}"#).unwrap();
        assert!(voices.is_empty());
        assert!(preferred_free_voice_id(&voices).is_none());
        assert_eq!(chinese_voice_count(&voices), 0);
        assert_eq!(english_voice_count(&voices), 0);
    }

    #[test]
    fn malformed_response_is_decode_error() {
        let error = interpret_voices_response(200, br#"{"not":"voices"}"#).unwrap_err();
        assert!(matches!(error, TtsError::Decode));
        let error = interpret_voices_response(200, b"not-json").unwrap_err();
        assert!(matches!(error, TtsError::Decode));
    }

    #[test]
    fn http_error_is_mapped() {
        let error = interpret_voices_response(401, br#"{"detail":{"status":"invalid_api_key"}}"#)
            .unwrap_err();
        assert!(matches!(error, TtsError::Unauthorized));
        let error = interpret_voices_response(500, b"server").unwrap_err();
        assert!(matches!(error, TtsError::Http { status: 500 }));
    }

    #[test]
    fn network_failure_is_preserved() {
        let error = finish_voices(Err(TtsError::Network)).unwrap_err();
        assert!(matches!(error, TtsError::Network));
    }

    #[test]
    #[ignore]
    fn live_elevenlabs_voice_list_stats() {
        dotenvy::dotenv().ok();
        let key = std::env::var("ELEVENLABS_API_KEY").unwrap_or_default();
        assert!(
            !key.trim().is_empty(),
            "ELEVENLABS_API_KEY is required for the live voice list check"
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (voices, stats) = runtime
            .block_on(fetch_elevenlabs_voices_with_stats(&key))
            .expect("ElevenLabs /v1/voices should succeed");
        eprintln!(
            "live voices: total={} default={} personal={} saved={} community={} free_available={} english={} chinese={} other={} picker={} preferred={}",
            stats.total,
            stats.default,
            stats.personal,
            stats.saved,
            stats.community,
            stats.free_available,
            stats.english,
            stats.chinese,
            stats.other,
            voices.len(),
            preferred_free_voice_id(&voices).unwrap_or_default()
        );
        assert!(!voices.is_empty() || stats.total == 0);
        assert!(!voices.iter().any(|voice| voice.combo_label().contains("台湾普通话")));
    }
}
