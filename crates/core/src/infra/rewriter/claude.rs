use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::prompts;
use super::{RewriteContext, RewriteError, Rewriter};

/// Claude API を使用したリライター
pub struct ClaudeRewriter {
    client: reqwest::Client,
    api_key: String,
}

#[derive(Serialize)]
struct MessageRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct MessageResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

impl ClaudeRewriter {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key }
    }
}

#[async_trait]
impl Rewriter for ClaudeRewriter {
    async fn rewrite(
        &self,
        text: &str,
        ctx: RewriteContext,
    ) -> Result<String, RewriteError> {
        let system_prompt = prompts::system_prompt_for_mode(&ctx.mode)
            .ok_or_else(|| RewriteError::NotAvailable("Raw mode does not support rewriting".to_string()))?;

        let (user_msg, _) = prompts::build_prompt(text, &ctx.dictionary_hints);

        let request = MessageRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 4096,
            system: system_prompt.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: user_msg,
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    RewriteError::Timeout
                } else {
                    RewriteError::Failed(format!("HTTP request failed: {e}"))
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RewriteError::Failed(format!(
                "Claude API error: {status} - {body}"
            )));
        }

        let msg_response: MessageResponse = response
            .json()
            .await
            .map_err(|e| RewriteError::Failed(format!("Response parse error: {e}")))?;

        let text = msg_response
            .content
            .into_iter()
            .filter_map(|block| block.text)
            .collect::<Vec<_>>()
            .join("");

        if text.is_empty() {
            return Err(RewriteError::Failed("Empty response from Claude API".to_string()));
        }

        Ok(text)
    }

    fn name(&self) -> &str {
        "claude"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_rewriter_name() {
        let rewriter = ClaudeRewriter::new("test-key".to_string());
        assert_eq!(rewriter.name(), "claude");
    }
}
