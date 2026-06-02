/// Integration tests for Hummingbird CLI end-to-end flows.
/// All tests use MockInferenceClient — no network calls or API keys required.
use async_trait::async_trait;
use hummingbird_agent::{Agent, MessageHistory};
use hummingbird_common::Result;
use hummingbird_inference::client::{InferenceClient, InferenceRequest, InferenceResponse, StreamToken};
use hummingbird_tools::{ReadFile, ToolRegistry, WriteFile};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::sync::mpsc;

// ── Mock inference client ─────────────────────────────────────────────────────

struct MockInferenceClient {
    responses: Mutex<Vec<String>>,
}

impl MockInferenceClient {
    fn new(responses: Vec<impl Into<String>>) -> Self {
        Self { responses: Mutex::new(responses.into_iter().map(|s| s.into()).collect()) }
    }
}

#[async_trait]
impl InferenceClient for MockInferenceClient {
    async fn send_message(&self, _req: InferenceRequest) -> Result<InferenceResponse> {
        let content = self.responses.lock().unwrap()
            .first()
            .cloned()
            .map(|s| { self.responses.lock().unwrap().remove(0); s })
            .unwrap_or_else(|| "Done.".to_string());
        Ok(InferenceResponse { content, model: "mock".into(), prompt_tokens: None, completion_tokens: None })
    }

    async fn stream_message(&self, _req: InferenceRequest, _tx: mpsc::Sender<Result<StreamToken>>) -> Result<()> {
        Ok(())
    }

    fn provider_name(&self) -> &str { "mock" }
}

fn make_agent(responses: Vec<&str>) -> Agent {
    let client = Arc::new(MockInferenceClient::new(responses));
    let mut registry = ToolRegistry::new();
    registry.register(ReadFile);
    registry.register(WriteFile);
    Agent::new(client, Arc::new(registry), "mock-model")
}

// ── Test scenarios ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_simple_question_answering() {
    let agent = make_agent(vec!["The answer is 42."]);
    let result = agent.run("What is the answer?").await.unwrap();
    assert_eq!(result.final_response, "The answer is 42.");
    assert_eq!(result.iterations, 1);
}

#[tokio::test]
async fn scenario_single_file_edit() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("hello.rs");
    std::fs::write(&file_path, "fn main() {}").unwrap();

    let write_call = format!(
        r#"<tool_call>{{"name":"write_file","arguments":{{"path":"{}","content":"fn main() {{ println!(\"hello\"); }}"}}}}</tool_call>"#,
        file_path.to_str().unwrap()
    );
    let agent = make_agent(vec![&write_call, "File updated successfully."]);
    let result = agent.run("Add println to main").await.unwrap();
    assert!(result.final_response.contains("successfully"));
}

#[tokio::test]
async fn scenario_multi_file_edit() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("a.rs"), "// a").unwrap();
    std::fs::write(dir.path().join("b.rs"), "// b").unwrap();

    let read_a = format!(
        r#"<tool_call>{{"name":"read_file","arguments":{{"path":"{}"}}}}</tool_call>"#,
        dir.path().join("a.rs").to_str().unwrap()
    );
    let read_b = format!(
        r#"<tool_call>{{"name":"read_file","arguments":{{"path":"{}"}}}}</tool_call>"#,
        dir.path().join("b.rs").to_str().unwrap()
    );
    let agent = make_agent(vec![&read_a, &read_b, "Both files read."]);
    let result = agent.run("Read both files").await.unwrap();
    assert_eq!(result.iterations, 3);
}

#[tokio::test]
async fn scenario_tool_error_recovery() {
    // Agent calls a tool that returns an error result (not a Rust error)
    let bad_read = r#"<tool_call>{"name":"read_file","arguments":{"path":"/nonexistent/path.rs"}}</tool_call>"#;
    let agent = make_agent(vec![bad_read, "I couldn't read the file, proceeding differently."]);
    let result = agent.run("Read /nonexistent/path.rs").await.unwrap();
    // Agent should recover and produce a final response
    assert!(!result.final_response.is_empty());
}

#[tokio::test]
async fn scenario_max_iteration_limit() {
    let infinite_call = r#"<tool_call>{"name":"read_file","arguments":{"path":"/tmp/x"}}</tool_call>"#;
    let responses: Vec<&str> = vec![infinite_call; 20];
    let mut agent = make_agent(responses);
    agent.max_iterations = 3;

    let result = agent.run("Loop").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("max iterations"));
}
