use hummingbird_common::{HummingbirdError, Result};
use hummingbird_inference::client::{InferenceClient, InferenceRequest};
use hummingbird_tools::ToolRegistry;
use serde_json::Value;
use std::sync::Arc;

use crate::history::MessageHistory;

const DEFAULT_MAX_ITERATIONS: u32 = 10;

pub struct Agent {
    pub client: Arc<dyn InferenceClient>,
    pub tools: Arc<ToolRegistry>,
    pub model: String,
    pub max_iterations: u32,
    pub max_tokens: usize,
    pub system_prompt: Option<String>,
}

pub struct AgentRunResult {
    pub final_response: String,
    pub iterations: u32,
    pub history: MessageHistory,
}

impl Agent {
    pub fn new(
        client: Arc<dyn InferenceClient>,
        tools: Arc<ToolRegistry>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            client,
            tools,
            model: model.into(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            max_tokens: 4096,
            system_prompt: None,
        }
    }

    pub async fn run(&self, prompt: &str) -> Result<AgentRunResult> {
        let mut history = MessageHistory::new();
        history.push_user(prompt);

        for iteration in 0..self.max_iterations {
            let mut messages = history.messages.clone();

            // Prepend system prompt as first user message if set
            if let Some(sys) = &self.system_prompt {
                messages.insert(0, hummingbird_inference::client::Message {
                    role: "system".into(),
                    content: sys.clone(),
                });
            }

            let request = InferenceRequest {
                messages,
                model: self.model.clone(),
                max_tokens: self.max_tokens,
                temperature: 0.2,
                stream: false,
            };

            let response = self.client.send_message(request).await?;
            let content = response.content.clone();

            // Parse tool calls from the response
            if let Some(tool_call) = self.parse_tool_call(&content) {
                history.push_assistant(&content);

                // Execute the tool
                let tool_result = self.execute_tool_call(&tool_call).await;
                let result_text = match &tool_result {
                    Ok(r) => r.output.clone(),
                    Err(e) => format!("Tool error: {e}"),
                };

                history.push_tool_result(&tool_call.name, &result_text);
            } else {
                // No tool call — final response
                history.push_assistant(&content);
                return Ok(AgentRunResult {
                    final_response: content,
                    iterations: iteration + 1,
                    history,
                });
            }
        }

        Err(HummingbirdError::Agent(format!(
            "Agent reached max iterations ({}) without completing",
            self.max_iterations
        )))
    }

    fn parse_tool_call(&self, content: &str) -> Option<ToolCall> {
        // Support both OpenAI function_call JSON and simple <tool> XML-like markers
        // Try JSON first
        if let Ok(v) = serde_json::from_str::<Value>(content) {
            if let (Some(name), Some(args)) = (
                v["name"].as_str(),
                v.get("arguments"),
            ) {
                let params = match args {
                    Value::String(s) => serde_json::from_str(s).unwrap_or(Value::Null),
                    other => other.clone(),
                };
                return Some(ToolCall { name: name.to_string(), params });
            }
        }

        // Try <tool_call> tag pattern
        if let Some(start) = content.find("<tool_call>") {
            if let Some(end) = content.find("</tool_call>") {
                let inner = &content[start + "<tool_call>".len()..end];
                if let Ok(v) = serde_json::from_str::<Value>(inner.trim()) {
                    if let Some(name) = v["name"].as_str() {
                        let params = v["arguments"].clone();
                        return Some(ToolCall { name: name.to_string(), params });
                    }
                }
            }
        }

        None
    }

    async fn execute_tool_call(&self, call: &ToolCall) -> Result<hummingbird_tools::ToolResult> {
        let tool = self.tools.get(&call.name).ok_or_else(|| {
            HummingbirdError::Tool(format!("Unknown tool: {}", call.name))
        })?;
        tool.execute(call.params.clone()).await
    }
}

#[derive(Debug)]
struct ToolCall {
    name: String,
    params: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use hummingbird_inference::client::{InferenceResponse, StreamToken};
    use tokio::sync::mpsc;

    struct MockClient {
        responses: std::sync::Mutex<Vec<String>>,
    }

    impl MockClient {
        fn new(responses: Vec<&str>) -> Self {
            Self { responses: std::sync::Mutex::new(responses.into_iter().map(String::from).collect()) }
        }
    }

    #[async_trait]
    impl InferenceClient for MockClient {
        async fn send_message(&self, _req: InferenceRequest) -> Result<InferenceResponse> {
            let mut responses = self.responses.lock().unwrap();
            let content = if responses.is_empty() {
                "Done.".to_string()
            } else {
                responses.remove(0)
            };
            Ok(InferenceResponse { content, model: "mock".into(), prompt_tokens: None, completion_tokens: None })
        }

        async fn stream_message(&self, _req: InferenceRequest, _tx: mpsc::Sender<Result<StreamToken>>) -> Result<()> {
            Ok(())
        }

        fn provider_name(&self) -> &str { "mock" }
    }

    fn make_agent(responses: Vec<&str>) -> Agent {
        Agent::new(
            Arc::new(MockClient::new(responses)),
            Arc::new(ToolRegistry::new()),
            "mock-model",
        )
    }

    #[tokio::test]
    async fn simple_completion_no_tool_calls() {
        let agent = make_agent(vec!["Hello from the agent!"]);
        let result = agent.run("Say hello").await.unwrap();
        assert_eq!(result.final_response, "Hello from the agent!");
        assert_eq!(result.iterations, 1);
    }

    #[tokio::test]
    async fn single_tool_use_then_final_response() {
        let tool_call_json = r#"<tool_call>{"name":"read_file","arguments":{"path":"/tmp/x"}}</tool_call>"#;
        let agent = make_agent(vec![tool_call_json, "File content processed."]);
        let result = agent.run("Read /tmp/x").await.unwrap();
        assert_eq!(result.final_response, "File content processed.");
        assert_eq!(result.iterations, 2);
    }

    #[tokio::test]
    async fn multi_step_tool_use() {
        let call1 = r#"<tool_call>{"name":"read_file","arguments":{"path":"/a"}}</tool_call>"#;
        let call2 = r#"<tool_call>{"name":"read_file","arguments":{"path":"/b"}}</tool_call>"#;
        let agent = make_agent(vec![call1, call2, "Both files processed."]);
        let result = agent.run("Process files").await.unwrap();
        assert_eq!(result.iterations, 3);
    }

    #[tokio::test]
    async fn max_iteration_stop() {
        // All responses are tool calls — should hit max_iterations
        let calls: Vec<&str> = vec![
            r#"<tool_call>{"name":"x","arguments":{}}</tool_call>"#; 15
        ];
        let mut agent = make_agent(calls);
        agent.max_iterations = 3;
        let result = agent.run("Loop forever").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max iterations"));
    }

    #[tokio::test]
    async fn maintains_message_history() {
        let agent = make_agent(vec!["Response 1"]);
        let result = agent.run("Prompt").await.unwrap();
        assert!(result.history.len() >= 2);
    }
}
