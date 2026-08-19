use std::{env, time::Duration};

use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};

use super::LlmError;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const CHAT_COMPLETIONS_PATH: &str = "chat/completions";
const MAX_COMPLETION_TOKENS: u32 = 180;
const REASONING_EFFORT_NONE: &str = "none";

#[derive(Clone)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout: Duration,
}

impl std::fmt::Debug for LlmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmConfig")
            .field("base_url", &self.base_url)
            .field("api_key", &"***")
            .field("model", &self.model)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl LlmConfig {
    pub fn from_env() -> Result<Self, LlmError> {
        dotenvy::dotenv().ok();

        let base_url = read_required_env("LLM_BASE_URL")?;
        let api_key = read_required_env("LLM_API_KEY")?;
        let model = read_required_env("LLM_MODEL")?;
        let timeout = env::var("LLM_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(DEFAULT_TIMEOUT_SECS));

        Ok(Self {
            base_url,
            api_key,
            model,
            timeout,
        })
    }
}

#[derive(Clone)]
pub struct LlmClient {
    http: Client,
    completions_url: Url,
    api_key: String,
    model: String,
}

impl std::fmt::Debug for LlmClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmClient")
            .field("completions_url", &self.completions_url)
            .field("api_key", &"***")
            .field("model", &self.model)
            .finish()
    }
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let normalized_base_url = format!("{}/", config.base_url.trim_end_matches('/'));
        let base_url = Url::parse(&normalized_base_url)
            .map_err(|_| LlmError::InvalidBaseUrl(config.base_url.clone()))?;
        let completions_url = base_url
            .join(CHAT_COMPLETIONS_PATH)
            .map_err(|_| LlmError::InvalidBaseUrl(config.base_url.clone()))?;
        let http = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|source| LlmError::ClientBuild { source })?;

        Ok(Self {
            http,
            completions_url,
            api_key: config.api_key,
            model: config.model,
        })
    }

    pub fn from_env() -> Result<Self, LlmError> {
        Self::new(LlmConfig::from_env()?)
    }

    pub async fn complete_prompt(&self, prompt: &str) -> Result<String, LlmError> {
        let request = commentary_chat_request(self.model.clone(), prompt);
        self.send_chat_completion(request).await
    }

    pub async fn generate_commentary(
        &self,
        all_game_data_json: &str,
    ) -> Result<String, LlmError> {
        let user_content = format!(
            "Riot Live Client allgamedata JSON:\n```json\n{}\n```",
            all_game_data_json
        );
        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: SYSTEM_PROMPT.to_string(),
                },
                ChatMessage {
                    role: "user",
                    content: user_content,
                },
            ],
            temperature: 0.7,
            max_tokens: MAX_COMPLETION_TOKENS,
            reasoning: ReasoningConfig::disabled(),
        };

        self.send_chat_completion(request).await
    }

    async fn send_chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<String, LlmError> {
        let url_string = self.completions_url.to_string();
        let response = self
            .http
            .post(self.completions_url.clone())
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|source| map_request_error(url_string.clone(), source))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::HttpStatus {
                url: url_string,
                status,
                body,
            });
        }

        let response = response
            .json::<ChatCompletionResponse>()
            .await
            .map_err(|source| LlmError::Decode {
                url: url_string,
                source,
            })?;

        print_llm_response_debug(status, &response);

        extract_message_content(response)
    }
}

const SYSTEM_PROMPT: &str = r#"You are a professional League of Legends esports commentator.

Generate exactly one concise spoken commentary line based only on the provided Riot Live Client allgamedata JSON.

Rules:
- Do not coach the player.
- Do not give advice or instructions.
- Do not infer hidden enemy information.
- Do not predict future events.
- Do not mention that you are reading JSON.
- If there is no obvious action, give a neutral current-state commentary line.
- Output only the commentary line."#;

fn commentary_chat_request(model: String, prompt: &str) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model,
        messages: vec![ChatMessage {
            role: "user",
            content: prompt.to_string(),
        }],
        temperature: 0.7,
        max_tokens: MAX_COMPLETION_TOKENS,
        reasoning: ReasoningConfig::disabled(),
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
    reasoning: ReasoningConfig,
}

#[derive(Debug, Serialize)]
struct ReasoningConfig {
    effort: &'static str,
}

impl ReasoningConfig {
    fn disabled() -> Self {
        Self {
            effort: REASONING_EFFORT_NONE,
        }
    }
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    #[serde(default)]
    id: Option<String>,

    #[serde(default)]
    model: Option<String>,

    #[serde(default)]
    usage: Option<ChatCompletionUsage>,

    choices: Vec<ChatCompletionChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChoice {
    #[serde(default)]
    index: Option<u32>,

    #[serde(rename = "finish_reason", default)]
    finish_reason: Option<String>,

    #[serde(rename = "native_finish_reason", default)]
    native_finish_reason: Option<String>,

    message: ChatCompletionMessage,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionUsage {
    #[serde(rename = "prompt_tokens", default)]
    prompt_tokens: Option<u32>,

    #[serde(rename = "completion_tokens", default)]
    completion_tokens: Option<u32>,

    #[serde(rename = "total_tokens", default)]
    total_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatCompletionMessage {
    content: Option<String>,

    #[serde(default)]
    reasoning: Option<String>,

    #[serde(default)]
    reasoning_details: Option<Vec<ReasoningDetail>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ReasoningDetail {
    #[serde(default)]
    r#type: Option<String>,

    #[serde(default)]
    text: Option<String>,

    #[serde(default)]
    signature: Option<String>,
}

fn read_required_env(name: &'static str) -> Result<String, LlmError> {
    env::var(name)
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(LlmError::MissingEnv { name })
}

fn map_request_error(url: String, source: reqwest::Error) -> LlmError {
    if source.is_timeout() {
        LlmError::Timeout { url, source }
    } else {
        LlmError::RequestFailed { url, source }
    }
}

fn extract_message_content(response: ChatCompletionResponse) -> Result<String, LlmError> {
    response
        .choices
        .into_iter()
        .find_map(|choice| {
            let content = choice.message.content?.trim().to_string();
            (!content.is_empty()).then_some(content)
        })
        .ok_or(LlmError::EmptyResponse)
}

fn print_llm_response_debug(status: reqwest::StatusCode, response: &ChatCompletionResponse) {
    println!("[LLM Response Debug]");
    println!("status code: {}", status.as_u16());
    println!("response id present: {}", response.id.is_some());
    println!("response model present: {}", response.model.is_some());
    println!("number of choices: {}", response.choices.len());

    if let Some(usage) = &response.usage {
        println!(
            "usage present: true, prompt_tokens: {:?}, completion_tokens: {:?}, total_tokens: {:?}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    } else {
        println!("usage present: false");
    }

    for (index, choice) in response.choices.iter().enumerate() {
        let content_empty = choice
            .message
            .content
            .as_deref()
            .map(str::trim)
            .map_or(true, str::is_empty);
        let reasoning_empty = choice
            .message
            .reasoning
            .as_deref()
            .map(str::trim)
            .map_or(true, str::is_empty);
        let reasoning_details_empty = choice
            .message
            .reasoning_details
            .as_ref()
            .map_or(true, Vec::is_empty);

        println!("choice[{index}].index: {:?}", choice.index);
        println!("choice[{index}].finish_reason: {:?}", choice.finish_reason);
        println!(
            "choice[{index}].native_finish_reason: {:?}",
            choice.native_finish_reason
        );
        println!("choice[{index}].message exists: true");
        println!("choice[{index}].message.content empty: {content_empty}");
        println!("choice[{index}].message.reasoning empty: {reasoning_empty}");
        println!(
            "choice[{index}].message.reasoning_details empty: {reasoning_details_empty}"
        );
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn required_env_rejects_empty_values() {
        env::set_var("LLM_TEST_EMPTY", " ");

        let result = read_required_env("LLM_TEST_EMPTY");

        assert!(matches!(result, Err(LlmError::MissingEnv { .. })));
        env::remove_var("LLM_TEST_EMPTY");
    }

    #[test]
    fn openrouter_null_content_response_decodes_without_failure() {
        let payload = json!({
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "reasoning": null,
                        "reasoning_details": null
                    }
                }
            ]
        });

        let response: ChatCompletionResponse =
            serde_json::from_value(payload).expect("OpenRouter response should decode");

        assert!(response.choices[0].message.content.is_none());
        assert!(response.choices[0].message.reasoning.is_none());
        assert!(response.choices[0].message.reasoning_details.is_none());
        assert!(matches!(
            extract_message_content(response),
            Err(LlmError::EmptyResponse)
        ));
    }

    #[test]
    fn chat_request_disables_openrouter_reasoning() {
        let request = commentary_chat_request("test-model".to_string(), "只输出一句中文解说。");
        let payload = serde_json::to_value(&request).expect("request should serialize");

        assert_eq!(payload["max_tokens"], MAX_COMPLETION_TOKENS);
        assert_eq!(payload["reasoning"]["effort"], REASONING_EFFORT_NONE);
        assert!(payload["reasoning"].get("max_tokens").is_none());
        assert!(payload["reasoning"].get("enabled").is_none());
    }

    #[test]
    fn commentary_comes_only_from_message_content() {
        let payload = json!({
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "蓝方开始集中。",
                        "reasoning": "The user wants me to act as a caster..."
                    }
                }
            ]
        });

        let response: ChatCompletionResponse =
            serde_json::from_value(payload).expect("OpenRouter response should decode");
        let content = extract_message_content(response).expect("content should exist");

        assert_eq!(content, "蓝方开始集中。");
        assert!(!content.contains("The user wants me"));
    }

    #[test]
    fn reasoning_only_response_returns_empty_response() {
        let payload = json!({
            "choices": [
                {
                    "finish_reason": "length",
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "reasoning": "internal reasoning only",
                        "reasoning_details": [
                            {
                                "type": "reasoning.text",
                                "text": "hidden chain"
                            }
                        ]
                    }
                }
            ]
        });

        let response: ChatCompletionResponse =
            serde_json::from_value(payload).expect("OpenRouter response should decode");

        assert!(matches!(
            extract_message_content(response),
            Err(LlmError::EmptyResponse)
        ));
    }

    #[test]
    fn skips_null_content_and_extracts_first_non_empty_content() {
        let payload = json!({
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "reasoning": "internal reasoning",
                        "reasoning_details": [
                            {
                                "type": "reasoning.text",
                                "text": "hidden chain"
                            }
                        ]
                    }
                },
                {
                    "message": {
                        "role": "assistant",
                        "content": "这是一句解说。"
                    }
                }
            ]
        });

        let response: ChatCompletionResponse =
            serde_json::from_value(payload).expect("OpenRouter response should decode");

        let content = extract_message_content(response).expect("content should exist");

        assert_eq!(content, "这是一句解说。");
    }
}
