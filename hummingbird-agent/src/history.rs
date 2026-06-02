use hummingbird_inference::client::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageHistory {
    pub messages: Vec<Message>,
}

impl MessageHistory {
    pub fn new() -> Self { Self::default() }

    pub fn push_user(&mut self, content: impl Into<String>) {
        self.messages.push(Message { role: "user".into(), content: content.into() });
    }

    pub fn push_assistant(&mut self, content: impl Into<String>) {
        self.messages.push(Message { role: "assistant".into(), content: content.into() });
    }

    pub fn push_tool_result(&mut self, tool_name: &str, result: &str) {
        self.messages.push(Message {
            role: "user".into(),
            content: format!("[Tool result: {tool_name}]\n{result}"),
        });
    }

    pub fn as_messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn len(&self) -> usize { self.messages.len() }
    pub fn is_empty(&self) -> bool { self.messages.is_empty() }
}
