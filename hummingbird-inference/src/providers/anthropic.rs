use async_trait::async_trait;
use futures::StreamExt;
use hummingbird_common::{HummingbirdError, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::client::{InferenceClient, InferenceRequest, InferenceResponse, StreamToken};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicClient {
    api_key: String,
    base_url: String,
    http: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: ANTHROPIC_API_URL.to_string(),
            http: reqwest::Client::new(),
        }
    }

    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| HummingbirdError::Config("ANTHROPIC_API_KEY not set".to_string()))?;
        Ok(Self::new(key))
    }
}

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    messages: Vec<serde_json::Value>,
    max_tokens: usize,
    stream: bool,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    model: String,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[async_trait]
impl InferenceClient for AnthropicClient {
    async fn send_message(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();

        let body = AnthropicRequest {
            model: &request.model,
            messages,
            max_tokens: request.max_tokens,
            stream: false,
        };

        let resp = self
            .http
            .post(&self.base_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await
            .map_err(|e| HummingbirdError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(HummingbirdError::Inference(format!(
                "Anthropic {status}: {text}"
            )));
        }

        let data: AnthropicResponse = resp
            .json()
            .await
            .map_err(|e| HummingbirdError::Inference(e.to_string()))?;

        let content = data
            .content
            .into_iter()
            .filter(|c| c.kind == "text")
            .filter_map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(InferenceResponse {
            content,
            model: data.model,
            prompt_tokens: data.usage.as_ref().map(|u| u.input_tokens),
            completion_tokens: data.usage.as_ref().map(|u| u.output_tokens),
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

        let body = AnthropicRequest {
            model: &request.model,
            messages,
            max_tokens: request.max_tokens,
            stream: true,
        };

        let resp = self
            .http
            .post(&self.base_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await
            .map_err(|e| HummingbirdError::Network(e.to_string()))?;

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| HummingbirdError::Network(e.to_string()))?;
            let text = String::from_utf8_lossy(&bytes);

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        let event_type = event["type"].as_str().unwrap_or("");
                        match event_type {
                            "content_block_delta" => {
                                if let Some(text) = event["delta"]["text"].as_str() {
                                    let _ = tx
                                        .send(Ok(StreamToken {
                                            text: text.to_string(),
                                            done: false,
                                        }))
                                        .await;
                                }
                            }
                            "message_stop" => {
                                let _ = tx
                                    .send(Ok(StreamToken {
                                        text: String::new(),
                                        done: true,
                                    }))
                                    .await;
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_provider_name() {
        let c = AnthropicClient::new("sk-test");
        assert_eq!(c.provider_name(), "anthropic");
    }

    #[test]
    fn from_env_errors_without_key() {
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert!(AnthropicClient::from_env().is_err());
    }
}
