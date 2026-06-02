use async_trait::async_trait;
use futures::StreamExt;
use hummingbird_common::{HummingbirdError, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::client::{InferenceClient, InferenceRequest, InferenceResponse, StreamToken};

pub struct OllamaClient {
    base_url: String,
    http: reqwest::Client,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    pub fn default_local() -> Self {
        Self::new("http://localhost:11434")
    }
}

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    messages: Vec<serde_json::Value>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    num_predict: usize,
    temperature: f32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
    model: String,
    done: bool,
    prompt_eval_count: Option<u32>,
    eval_count: Option<u32>,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}

#[async_trait]
impl InferenceClient for OllamaClient {
    async fn send_message(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();

        let body = OllamaRequest {
            model: &request.model,
            messages,
            stream: false,
            options: OllamaOptions {
                num_predict: request.max_tokens,
                temperature: request.temperature,
            },
        };

        let resp = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| HummingbirdError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(HummingbirdError::Inference(format!(
                "Ollama {status}: {text}"
            )));
        }

        let data: OllamaResponse = resp
            .json()
            .await
            .map_err(|e| HummingbirdError::Inference(e.to_string()))?;

        Ok(InferenceResponse {
            content: data.message.content,
            model: data.model,
            prompt_tokens: data.prompt_eval_count,
            completion_tokens: data.eval_count,
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

        let body = OllamaRequest {
            model: &request.model,
            messages,
            stream: true,
            options: OllamaOptions {
                num_predict: request.max_tokens,
                temperature: request.temperature,
            },
        };

        let resp = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| HummingbirdError::Network(e.to_string()))?;

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| HummingbirdError::Network(e.to_string()))?;
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines() {
                if line.is_empty() {
                    continue;
                }
                if let Ok(data) = serde_json::from_str::<OllamaResponse>(line) {
                    let done = data.done;
                    let _ = tx
                        .send(Ok(StreamToken {
                            text: data.message.content,
                            done,
                        }))
                        .await;
                    if done {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_local_url() {
        let c = OllamaClient::default_local();
        assert!(c.base_url.contains("11434"));
        assert_eq!(c.provider_name(), "ollama");
    }
}
