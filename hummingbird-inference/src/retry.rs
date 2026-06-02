use async_trait::async_trait;
use hummingbird_common::{HummingbirdError, Result};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::client::{InferenceClient, InferenceRequest, InferenceResponse, StreamToken};

const MAX_RETRIES: u32 = 3;
const BASE_BACKOFF_MS: u64 = 500;

pub struct RetryClient<C: InferenceClient> {
    inner: C,
    max_retries: u32,
}

impl<C: InferenceClient> RetryClient<C> {
    pub fn new(inner: C) -> Self {
        Self { inner, max_retries: MAX_RETRIES }
    }

    pub fn with_retries(inner: C, max_retries: u32) -> Self {
        Self { inner, max_retries }
    }

    fn is_retryable(err: &HummingbirdError) -> bool {
        match err {
            HummingbirdError::Inference(msg) => {
                msg.contains("429") || msg.contains("500") || msg.contains("502")
                    || msg.contains("503") || msg.contains("504")
            }
            HummingbirdError::Network(_) => true,
            _ => false,
        }
    }
}

#[async_trait]
impl<C: InferenceClient + Sync> InferenceClient for RetryClient<C> {
    async fn send_message(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let mut last_err = None;
        for attempt in 0..=self.max_retries {
            match self.inner.send_message(request.clone()).await {
                Ok(resp) => return Ok(resp),
                Err(e) if attempt < self.max_retries && Self::is_retryable(&e) => {
                    let delay = BASE_BACKOFF_MS * 2u64.pow(attempt);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    last_err = Some(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap())
    }

    async fn stream_message(
        &self,
        request: InferenceRequest,
        tx: mpsc::Sender<Result<StreamToken>>,
    ) -> Result<()> {
        self.inner.stream_message(request, tx).await
    }

    fn provider_name(&self) -> &str {
        self.inner.provider_name()
    }
}

// ── Rate limiter ──────────────────────────────────────────────────────────────

pub struct RateLimiter {
    requests_per_minute: u32,
    timestamps: Arc<Mutex<Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        Self {
            requests_per_minute,
            timestamps: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn acquire(&self) {
        loop {
            let now = Instant::now();
            let window = Duration::from_secs(60);
            let mut stamps = self.timestamps.lock().unwrap();
            stamps.retain(|t| now.duration_since(*t) < window);
            if stamps.len() < self.requests_per_minute as usize {
                stamps.push(now);
                return;
            }
            // oldest stamp + window gives next available slot
            let oldest = stamps[0];
            let wait = (oldest + window).saturating_duration_since(now);
            drop(stamps);
            tokio::time::sleep(wait + Duration::from_millis(10)).await;
        }
    }
}

// ── Token counting ────────────────────────────────────────────────────────────

/// Rough approximation: ~4 chars per token (GPT tokenizer heuristic).
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

pub fn estimate_request_tokens(request: &InferenceRequest) -> usize {
    request.messages.iter().map(|m| estimate_tokens(&m.content) + 4).sum::<usize>() + 3
}

pub fn check_context_window(request: &InferenceRequest, limit: usize) -> Result<()> {
    let tokens = estimate_request_tokens(request);
    if tokens > limit {
        return Err(HummingbirdError::TokenLimitExceeded { requested: tokens, limit });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Message;

    fn make_request(content: &str) -> InferenceRequest {
        InferenceRequest::new(
            vec![Message { role: "user".into(), content: content.into() }],
            "test-model",
        )
    }

    #[test]
    fn token_estimate_non_zero() {
        assert!(estimate_tokens("hello world") > 0);
    }

    #[test]
    fn context_window_passes_within_limit() {
        let req = make_request("hello");
        assert!(check_context_window(&req, 10000).is_ok());
    }

    #[test]
    fn context_window_rejects_oversized() {
        let long = "x".repeat(40001); // ~10000 tokens
        let req = make_request(&long);
        assert!(check_context_window(&req, 100).is_err());
    }

    #[test]
    fn rate_limiter_created() {
        let rl = RateLimiter::new(60);
        assert_eq!(rl.requests_per_minute, 60);
    }

    #[test]
    fn is_retryable_on_429() {
        let err = HummingbirdError::Inference("OpenAI 429: rate limited".to_string());
        assert!(RetryClient::<crate::providers::OllamaClient>::is_retryable(&err));
    }

    #[test]
    fn is_not_retryable_on_config_error() {
        let err = HummingbirdError::Config("bad key".to_string());
        assert!(!RetryClient::<crate::providers::OllamaClient>::is_retryable(&err));
    }
}
