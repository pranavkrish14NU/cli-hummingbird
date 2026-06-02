use async_trait::async_trait;
use hummingbird_common::{HummingbirdError, Result};
use serde_json::{json, Value};

use crate::tool::{Tool, ToolResult};

pub struct ReadFile;

#[async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &str { "read_file" }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute or relative path to the file" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let path = params["path"].as_str()
            .ok_or_else(|| HummingbirdError::Tool("missing 'path' parameter".into()))?;

        match std::fs::read_to_string(path) {
            Ok(content) => Ok(ToolResult::ok(content)),
            Err(e) => Ok(ToolResult::err(format!("Failed to read '{path}': {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn reads_existing_file() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("hello.txt");
        fs::write(&p, "hello world").unwrap();
        let result = ReadFile.execute(json!({"path": p.to_str().unwrap()})).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.output, "hello world");
    }

    #[tokio::test]
    async fn returns_error_for_missing_file() {
        let result = ReadFile.execute(json!({"path": "/nonexistent/file.txt"})).await.unwrap();
        assert!(result.is_error);
    }
}
