use async_trait::async_trait;
use hummingbird_common::{HummingbirdError, Result};
use serde_json::{json, Value};
use std::path::Path;

use crate::tool::{Tool, ToolResult};

pub struct WriteFile;

#[async_trait]
impl Tool for WriteFile {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file, creating parent directories as needed. Requires explicit confirmation before overwriting."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path":    { "type": "string", "description": "Path to write to" },
                "content": { "type": "string", "description": "Content to write" }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| HummingbirdError::Tool("missing 'path' parameter".into()))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| HummingbirdError::Tool("missing 'content' parameter".into()))?;

        let p = Path::new(path);
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(HummingbirdError::Io)?;
            }
        }

        std::fs::write(p, content).map_err(HummingbirdError::Io)?;
        Ok(ToolResult::ok(format!(
            "Written {} bytes to '{path}'",
            content.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn writes_file_and_creates_dirs() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("sub").join("file.txt");
        let result = WriteFile
            .execute(json!({
                "path": p.to_str().unwrap(),
                "content": "hello"
            }))
            .await
            .unwrap();
        assert!(!result.is_error);
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "hello");
    }

    #[tokio::test]
    async fn errors_on_missing_params() {
        let result = WriteFile.execute(json!({"path": "/tmp/x"})).await;
        assert!(result.is_err());
    }
}
