use async_trait::async_trait;
use hummingbird_common::{HummingbirdError, Result};
use serde_json::{json, Value};

use crate::tool::{Tool, ToolResult};

pub struct ListDirectory;

#[async_trait]
impl Tool for ListDirectory {
    fn name(&self) -> &str { "list_directory" }

    fn description(&self) -> &str {
        "List files and directories at the given path."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path to list" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let path = params["path"].as_str()
            .ok_or_else(|| HummingbirdError::Tool("missing 'path' parameter".into()))?;

        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(e) => return Ok(ToolResult::err(format!("Cannot list '{path}': {e}"))),
        };

        let mut lines: Vec<String> = entries
            .filter_map(|e| e.ok())
            .map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    format!("{name}/")
                } else {
                    name
                }
            })
            .collect();

        lines.sort();
        Ok(ToolResult::ok(lines.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn lists_directory_entries() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "").unwrap();
        fs::write(dir.path().join("b.txt"), "").unwrap();
        let result = ListDirectory.execute(json!({"path": dir.path().to_str().unwrap()})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.output.contains("a.txt"));
        assert!(result.output.contains("b.txt"));
    }

    #[tokio::test]
    async fn returns_error_for_missing_dir() {
        let result = ListDirectory.execute(json!({"path": "/nonexistent/dir"})).await.unwrap();
        assert!(result.is_error);
    }
}
