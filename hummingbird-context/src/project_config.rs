use hummingbird_common::{HummingbirdError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

const CONFIG_FILE: &str = ".hummingbird.toml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectContextConfig {
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub max_file_size_bytes: Option<usize>,
    pub max_total_tokens: Option<usize>,
}

impl ProjectContextConfig {
    pub fn load(root: &Path) -> Result<Option<Self>> {
        let cfg_path = root.join(CONFIG_FILE);
        if !cfg_path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&cfg_path).map_err(HummingbirdError::Io)?;
        let cfg: Self = toml::from_str(&content)
            .map_err(|e| HummingbirdError::Config(format!("Failed to parse {CONFIG_FILE}: {e}")))?;
        Ok(Some(cfg))
    }

    /// Merge config file settings with CLI overrides.
    /// CLI values (non-None) take precedence over config file values.
    pub fn merge_with_cli(
        &self,
        cli_include: Option<Vec<String>>,
        cli_exclude: Option<Vec<String>>,
        cli_max_size: Option<usize>,
        cli_max_tokens: Option<usize>,
    ) -> ResolvedContextConfig {
        ResolvedContextConfig {
            include: cli_include
                .or_else(|| self.include.clone())
                .unwrap_or_else(|| vec!["**/*".to_string()]),
            exclude: cli_exclude
                .or_else(|| self.exclude.clone())
                .unwrap_or_else(|| vec!["target/**".to_string(), ".git/**".to_string()]),
            max_file_size_bytes: cli_max_size
                .or(self.max_file_size_bytes)
                .unwrap_or(1_048_576),
            max_total_tokens: cli_max_tokens
                .or(self.max_total_tokens)
                .unwrap_or(64_000),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedContextConfig {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub max_file_size_bytes: usize,
    pub max_total_tokens: usize,
}

impl Default for ResolvedContextConfig {
    fn default() -> Self {
        ProjectContextConfig::default().merge_with_cli(None, None, None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn loads_config_from_file() {
        let dir = TempDir::new().unwrap();
        let content = r#"
include = ["**/*.rs"]
exclude = ["target/**"]
max_file_size_bytes = 512000
"#;
        fs::write(dir.path().join(".hummingbird.toml"), content).unwrap();
        let cfg = ProjectContextConfig::load(dir.path()).unwrap().unwrap();
        assert_eq!(cfg.include.as_deref(), Some(&["**/*.rs".to_string()][..]));
        assert_eq!(cfg.max_file_size_bytes, Some(512000));
    }

    #[test]
    fn returns_none_when_config_missing() {
        let dir = TempDir::new().unwrap();
        let result = ProjectContextConfig::load(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn cli_overrides_config_file() {
        let cfg = ProjectContextConfig {
            include: Some(vec!["**/*.rs".to_string()]),
            exclude: None,
            max_file_size_bytes: Some(100),
            max_total_tokens: None,
        };
        let resolved = cfg.merge_with_cli(
            Some(vec!["**/*.py".to_string()]),
            None,
            None,
            None,
        );
        assert_eq!(resolved.include, vec!["**/*.py".to_string()]);
        assert_eq!(resolved.max_file_size_bytes, 100); // from file since CLI is None
    }

    #[test]
    fn defaults_applied_when_no_config_and_no_cli() {
        let cfg = ProjectContextConfig::default();
        let resolved = cfg.merge_with_cli(None, None, None, None);
        assert_eq!(resolved.max_file_size_bytes, 1_048_576);
        assert_eq!(resolved.max_total_tokens, 64_000);
    }
}
