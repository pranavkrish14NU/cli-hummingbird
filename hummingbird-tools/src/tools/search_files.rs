use async_trait::async_trait;
use hummingbird_common::{HummingbirdError, Result};
use regex::Regex;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::tool::{Tool, ToolResult};

pub struct SearchFiles;

#[async_trait]
impl Tool for SearchFiles {
    fn name(&self) -> &str { "search_files" }

    fn description(&self) -> &str {
        "Search file contents using a regex pattern. Returns matching lines with file:line context."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "root":    { "type": "string", "description": "Root directory to search" },
                "pattern": { "type": "string", "description": "Regex pattern to search for" },
                "glob":    { "type": "string", "description": "Optional file glob filter (e.g. *.rs)" }
            },
            "required": ["root", "pattern"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let root = params["root"].as_str()
            .ok_or_else(|| HummingbirdError::Tool("missing 'root' parameter".into()))?;
        let pattern = params["pattern"].as_str()
            .ok_or_else(|| HummingbirdError::Tool("missing 'pattern' parameter".into()))?;
        let glob_filter = params["glob"].as_str();

        let re = Regex::new(pattern)
            .map_err(|e| HummingbirdError::Tool(format!("Invalid regex: {e}")))?;

        let mut matches: Vec<String> = Vec::new();

        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() { continue; }

            if let Some(glob) = glob_filter {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let pat = glob::Pattern::new(glob).unwrap_or_else(|_| glob::Pattern::new("*").unwrap());
                if !pat.matches(&name) { continue; }
            }

            if let Ok(content) = std::fs::read_to_string(path) {
                for (i, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        matches.push(format!("{}:{}: {}", path.display(), i + 1, line));
                    }
                }
            }
        }

        if matches.is_empty() {
            Ok(ToolResult::ok("No matches found."))
        } else {
            Ok(ToolResult::ok(matches.join("\n")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn finds_matching_lines() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {\n    println!(\"hello\");\n}").unwrap();
        let result = SearchFiles.execute(json!({
            "root": dir.path().to_str().unwrap(),
            "pattern": "fn main"
        })).await.unwrap();
        assert!(!result.is_error);
        assert!(result.output.contains("fn main"));
    }

    #[tokio::test]
    async fn returns_no_matches_message() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "hello").unwrap();
        let result = SearchFiles.execute(json!({
            "root": dir.path().to_str().unwrap(),
            "pattern": "ZZZNOTFOUND"
        })).await.unwrap();
        assert!(result.output.contains("No matches"));
    }

    #[tokio::test]
    async fn rejects_invalid_regex() {
        let result = SearchFiles.execute(json!({
            "root": ".",
            "pattern": "["
        })).await;
        assert!(result.is_err());
    }
}
