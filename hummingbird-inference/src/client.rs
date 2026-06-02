use async_trait::async_trait;
use hummingbird_common::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub messages: Vec<Message>,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub stream: bool,
}

impl InferenceRequest {
    pub fn new(messages: Vec<Message>, model: impl Into<String>) -> Self {
        Self {
            messages,
            model: model.into(),
            max_tokens: 4096,
            temperature: 0.2,
            stream: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub content: String,
    pub model: String,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct StreamToken {
    pub text: String,
    pub done: bool,
}

#[async_trait]
pub trait InferenceClient: Send + Sync {
    async fn send_message(&self, request: InferenceRequest) -> Result<InferenceResponse>;
    async fn stream_message(
        &self,
        request: InferenceRequest,
        tx: mpsc::Sender<Result<StreamToken>>,
    ) -> Result<()>;
    fn provider_name(&self) -> &str;
}
