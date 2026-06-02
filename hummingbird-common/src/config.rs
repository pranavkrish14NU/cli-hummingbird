use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub model: ModelConfig,
    pub context: ContextSettings,
    pub tools: ToolPaths,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub model_name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub max_tokens: usize,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSettings {
    pub max_file_size_bytes: usize,
    pub max_total_tokens: usize,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPaths {
    pub workspace_root: Option<String>,
    pub shell_timeout_secs: u64,
    pub blocked_commands: Vec<String>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig {
                provider: "ollama".to_string(),
                model_name: "qwen3:30b".to_string(),
                api_key: None,
                base_url: Some("http://localhost:11434".to_string()),
                max_tokens: 8192,
                temperature: 0.2,
            },
            context: ContextSettings {
                max_file_size_bytes: 1_048_576, // 1MB
                max_total_tokens: 64_000,
                include_patterns: vec!["**/*.rs".to_string(), "**/*.toml".to_string()],
                exclude_patterns: vec!["target/**".to_string(), ".git/**".to_string()],
            },
            tools: ToolPaths {
                workspace_root: None,
                shell_timeout_secs: 60,
                blocked_commands: vec![
                    "rm -rf /".to_string(),
                    "sudo".to_string(),
                    "mkfs".to_string(),
                    "dd".to_string(),
                ],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sensible_values() {
        let cfg = GlobalConfig::default();
        assert_eq!(cfg.model.provider, "ollama");
        assert_eq!(cfg.context.max_file_size_bytes, 1_048_576);
        assert!(!cfg.tools.blocked_commands.is_empty());
    }

    #[test]
    fn config_serializes_to_toml() {
        let cfg = GlobalConfig::default();
        let serialized = toml::to_string(&cfg).expect("should serialize");
        assert!(serialized.contains("provider"));
    }

    #[test]
    fn config_round_trips_toml() {
        let cfg = GlobalConfig::default();
        let toml_str = toml::to_string(&cfg).unwrap();
        let restored: GlobalConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(restored.model.provider, cfg.model.provider);
        assert_eq!(restored.context.max_total_tokens, cfg.context.max_total_tokens);
    }
}
