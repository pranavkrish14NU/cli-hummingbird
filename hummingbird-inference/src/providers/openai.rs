use async_trait::async_trait;
use futures::StreamExt;
use hummingbird_common::{HummingbirdError, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::client::{InferenceClient, InferenceRequest, InferenceResponse, StreamToken};

pub struct OpenAiClient {
    api_key: String,
    base_url: String,
    http: reqwest::Client,
}

impl OpenAiClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_base_url(api_key, "https://api.openai.com/v1")
    }

    pub fn from_env() -> Result<Self> {
        let key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| HummingbirdError::Config("OPENAI_API_KEY not set".to_string()))?;
        Ok(Self::new(key))
    }

    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: &'a [serde_json::Value],
    max_tokens: usize,
    temperature: f32,
    stream: bool,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    model: String,
    usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiMessage>,
    delta: Option<OpenAiMessage>,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[async_trait]
impl InferenceClient for OpenAiClient {
    async fn send_message(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();

        let body = OpenAiRequest {
            model: &request.model,
            messages: &messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: false,
        };

        let resp = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| HummingbirdError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(HummingbirdError::Inference(format!(
                "OpenAI {status}: {text}"
            )));
        }

        let data: OpenAiResponse = resp
            .json()
            .await
            .map_err(|e| HummingbirdError::Inference(e.to_string()))?;

        let content = data
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message)
            .and_then(|m| m.content)
            .unwrap_or_default();

        Ok(InferenceResponse {
            content,
            model: data.model,
            prompt_tokens: data.usage.as_ref().map(|u| u.prompt_tokens),
            completion_tokens: data.usage.as_ref().map(|u| u.completion_tokens),
        })
    }

    async fn stream_message(
        &self,
        request: InferenceRequest,
        tx: mpsc::Sender<Result<StreamToken>>,
    ) -> Result<()> {
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();

        let body = OpenAiRequest {
            model: &request.model,
            messages: &messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: true,
        };

        let resp = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| HummingbirdError::Network(e.to_string()))?;

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| HummingbirdError::Network(e.to_string()))?;
            let text = String::from_utf8_lossy(&bytes);

            for line in text.lines() {
                let line = line.trim_start_matches("data: ");
                if line == "[DONE]" {
                    let _ = tx
                        .send(Ok(StreamToken {
                            text: String::new(),
                            done: true,
                        }))
                        .await;
                    return Ok(());
                }
                if line.is_empty() {
                    continue;
                }
                if let Ok(data) = serde_json::from_str::<OpenAiResponse>(line) {
                    if let Some(delta_text) = data
                        .choices
                        .into_iter()
                        .next()
                        .and_then(|c| c.delta)
                        .and_then(|d| d.content)
                    {
                        let _ = tx
                            .send(Ok(StreamToken {
                                text: delta_text,
                                done: false,
                            }))
                            .await;
                    }
                }
            }
        }
        Ok(())
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Message;

    #[allow(dead_code)]
    fn mock_client(base_url: &str) -> OpenAiClient {
        OpenAiClient::with_base_url("test-key", base_url)
    }

    #[test]
    fn client_created_with_api_key() {
        let c = OpenAiClient::new("sk-test");
        assert_eq!(c.provider_name(), "openai");
    }

    #[test]
    fn from_env_errors_without_key() {
        std::env::remove_var("OPENAI_API_KEY");
        assert!(OpenAiClient::from_env().is_err());
    }

    #[test]
    fn inference_request_defaults() {
        let req = InferenceRequest::new(
            vec![Message {
                role: "user".into(),
                content: "hello".into(),
            }],
            "gpt-4o",
        );
        assert_eq!(req.max_tokens, 4096);
        assert!(!req.stream);
    }
}
