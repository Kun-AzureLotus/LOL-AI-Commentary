use std::{
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD, Engine as _};

use super::{commentary_to_ssml, TtsConfig, TtsEngine, TtsError, TtsUtterance};

#[derive(Clone)]
pub struct LocalTtsEngine {
    config: TtsConfig,
    current_child: Arc<Mutex<Option<std::process::Child>>>,
}

impl LocalTtsEngine {
    pub fn new() -> Self {
        Self::with_config(TtsConfig::default())
    }

    pub fn with_config(config: TtsConfig) -> Self {
        Self {
            config,
            current_child: Arc::new(Mutex::new(None)),
        }
    }

    pub fn config(&self) -> &TtsConfig {
        &self.config
    }
}

impl Default for LocalTtsEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub fn list_installed_voices() -> Vec<crate::tts::InstalledVoice> {
    #[cfg(not(windows))]
    {
        Vec::new()
    }

    #[cfg(windows)]
    {
        list_installed_voices_windows()
    }
}

#[cfg(windows)]
fn list_installed_voices_windows() -> Vec<crate::tts::InstalledVoice> {
    let script = r#"
Add-Type -AssemblyName System.Speech
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$voices = @($synth.GetInstalledVoices() | Where-Object { $_.Enabled } | ForEach-Object {
    [pscustomobject]@{
        name = $_.VoiceInfo.Name
        culture = $_.VoiceInfo.Culture.Name
        gender = $_.VoiceInfo.Gender.ToString()
    }
})
$voices | ConvertTo-Json -Compress
"#;
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_voice_json(&String::from_utf8_lossy(&output.stdout))
}

fn parse_voice_json(json: &str) -> Vec<crate::tts::InstalledVoice> {
    let json = json.trim();
    if json.is_empty() {
        return Vec::new();
    }

    #[derive(serde::Deserialize)]
    struct VoiceJson {
        name: String,
        culture: String,
        #[serde(default)]
        gender: Option<String>,
    }

    fn map_voice(voice: VoiceJson) -> crate::tts::InstalledVoice {
        crate::tts::InstalledVoice {
            name: voice.name,
            culture: voice.culture,
            gender: match voice
                .gender
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str()
            {
                "female" => crate::tts::VoiceGender::Female,
                "male" => crate::tts::VoiceGender::Male,
                _ => crate::tts::VoiceGender::Unknown,
            },
        }
    }

    if let Ok(voices) = serde_json::from_str::<Vec<VoiceJson>>(json) {
        return voices.into_iter().map(map_voice).collect();
    }

    if let Ok(voice) = serde_json::from_str::<VoiceJson>(json) {
        return vec![map_voice(voice)];
    }

    Vec::new()
}

impl TtsEngine for LocalTtsEngine {
    async fn speak(&self, utterance: &TtsUtterance) -> Result<(), TtsError> {
        if utterance.text.trim().is_empty() {
            return Ok(());
        }

        let engine = self.clone();
        let utterance = utterance.clone();
        match tokio::task::spawn_blocking(move || engine.speak_blocking(&utterance)).await {
            Ok(result) => result,
            Err(_) => Err(TtsError::PlaybackFailed),
        }
    }

    fn interrupt(&self) {
        if let Ok(mut current) = self.current_child.lock() {
            if let Some(mut child) = current.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl LocalTtsEngine {
    fn speak_blocking(&self, utterance: &TtsUtterance) -> Result<(), TtsError> {
        #[cfg(not(windows))]
        {
            let _ = utterance;
            Err(TtsError::UnsupportedPlatform)
        }

        #[cfg(windows)]
        {
            let script = windows_sapi_script(&self.config, utterance);
            let child = Command::new("powershell.exe")
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
                .spawn()
                .map_err(|_| TtsError::SpawnFailed)?;

            {
                let mut current = self
                    .current_child
                    .lock()
                    .map_err(|_| TtsError::PlaybackFailed)?;
                *current = Some(child);
            }

            loop {
                thread::sleep(Duration::from_millis(50));
                let mut current = self
                    .current_child
                    .lock()
                    .map_err(|_| TtsError::PlaybackFailed)?;
                let Some(child) = current.as_mut() else {
                    return Ok(());
                };
                match child.try_wait() {
                    Ok(Some(status)) => {
                        *current = None;
                        return if status.success() {
                            Ok(())
                        } else {
                            Err(TtsError::PlaybackFailed)
                        };
                    }
                    Ok(None) => {}
                    Err(_) => {
                        *current = None;
                        return Err(TtsError::PlaybackFailed);
                    }
                }
            }
        }
    }
}

pub(crate) fn windows_sapi_script(config: &TtsConfig, utterance: &TtsUtterance) -> String {
    let text = utterance.text.trim();
    let ssml = commentary_to_ssml(
        text,
        config.comma_pause_ms,
        config.sentence_pause_ms,
    );
    let text_b64 = STANDARD.encode(text.as_bytes());
    let ssml_b64 = STANDARD.encode(ssml.as_bytes());
    let preferred = config
        .voice_name
        .as_deref()
        .unwrap_or("")
        .replace('\'', "''");
    let rate = TtsConfig::clamp_rate(utterance.rate);
    let volume = TtsConfig::clamp_volume(utterance.volume);

    format!(
        r#"
Add-Type -AssemblyName System.Speech
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$text = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('{text_b64}'))
$ssml = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('{ssml_b64}'))
$preferred = '{preferred}'
$voices = @($synth.GetInstalledVoices() | Where-Object {{ $_.Enabled }})
$selected = $null
if ($preferred -ne '') {{
    $selected = $voices | Where-Object {{ $_.VoiceInfo.Name -eq $preferred }} | Select-Object -First 1
}}
if ($null -eq $selected) {{
    $selected = $voices | Where-Object {{ $_.VoiceInfo.Culture.Name -eq 'zh-CN' }} | Select-Object -First 1
}}
if ($null -eq $selected) {{
    $selected = $voices | Where-Object {{ $_.VoiceInfo.Culture.Name -like 'zh*' }} | Select-Object -First 1
}}
if ($null -ne $selected) {{
    $synth.SelectVoice($selected.VoiceInfo.Name)
}}
$synth.Rate = {rate}
$synth.Volume = {volume}
try {{
    $synth.SpeakSsml($ssml)
}} catch {{
    $synth.Speak($text)
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_text_does_not_play() {
        let engine = LocalTtsEngine::new();

        let result = engine.speak(&TtsUtterance::from_text("   ")).await;

        assert!(result.is_ok());
    }

    #[test]
    fn ssml_failure_falls_back_to_plain_speak() {
        let script = windows_sapi_script(
            &TtsConfig::default(),
            &TtsUtterance::from_text("蓝方开始集中。"),
        );

        assert!(script.contains("SpeakSsml"));
        assert!(script.contains("catch"));
        assert!(script.contains("$synth.Speak($text)"));
        assert!(script.contains("zh-CN"));
        assert!(script.contains("zh*"));
        assert!(!script.contains("蓝方开始集中。"));
    }

    #[test]
    fn missing_voice_does_not_hardcode_a_name() {
        let mut config = TtsConfig::default();
        config.voice_name = Some("Definitely Missing Voice".to_string());
        let script = windows_sapi_script(&config, &TtsUtterance::from_text("测试"));

        assert!(script.contains("Definitely Missing Voice"));
        assert!(script.contains("if ($null -ne $selected)"));
    }

    #[test]
    fn parse_voice_json_accepts_array_and_object() {
        let array = r#"[{"name":"Huihui","culture":"zh-CN"}]"#;
        let object = r#"{"name":"Huihui","culture":"zh-CN"}"#;

        assert_eq!(parse_voice_json(array)[0].culture, "zh-CN");
        assert_eq!(parse_voice_json(object)[0].name, "Huihui");
        assert!(parse_voice_json("").is_empty());
    }
}
