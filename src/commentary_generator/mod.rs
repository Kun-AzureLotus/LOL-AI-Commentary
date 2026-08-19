use thiserror::Error;

use crate::llm::{LlmClient, LlmConfig, LlmError};

pub async fn generate_commentary(prompt: &str) -> Result<String, CommentaryError> {
    generate_commentary_with_config(&LlmConfig::from_env()?, prompt).await
}

pub async fn generate_commentary_with_config(
    config: &LlmConfig,
    prompt: &str,
) -> Result<String, CommentaryError> {
    if prompt.trim().is_empty() {
        return Err(CommentaryError::EmptyPrompt);
    }

    let llm_client = LlmClient::new(config.clone())?;
    let commentary = llm_client.complete_prompt(prompt).await?;
    Ok(commentary)
}

#[derive(Debug, Error)]
pub enum CommentaryError {
    #[error("commentary prompt cannot be empty")]
    EmptyPrompt,

    #[error(transparent)]
    Llm {
        #[from]
        source: LlmError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_empty_prompt_before_calling_llm() {
        let result = generate_commentary("   ").await;

        assert!(matches!(result, Err(CommentaryError::EmptyPrompt)));
    }
}
