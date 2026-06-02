use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub max_file_size_bytes: usize,
    pub max_total_tokens: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            include: vec!["**/*.rs".to_string(), "**/*.toml".to_string()],
            exclude: vec!["target/**".to_string(), ".git/**".to_string()],
            max_file_size_bytes: 1_048_576,
            max_total_tokens: 64_000,
        }
    }
}
