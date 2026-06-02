use hummingbird_common::HummingbirdError;
use hummingbird_common::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub content: String,
    pub size_bytes: usize,
}

#[derive(Debug, Default)]
pub struct ContextBundle {
    pub files: Vec<FileEntry>,
    pub skipped: Vec<PathBuf>,
    pub total_bytes: usize,
}

pub struct ContextGatherer {
    pub max_file_size: usize,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

impl Default for ContextGatherer {
    fn default() -> Self {
        Self {
            max_file_size: 1_048_576,
            include_patterns: vec!["**/*".to_string()],
            exclude_patterns: vec!["target/**".to_string(), ".git/**".to_string()],
        }
    }
}

impl ContextGatherer {
    pub fn new(
        max_file_size: usize,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
    ) -> Self {
        Self {
            max_file_size,
            include_patterns,
            exclude_patterns,
        }
    }

    pub fn gather(&self, root: &Path, patterns: &[String]) -> Result<ContextBundle> {
        let mut bundle = ContextBundle::default();
        let effective_patterns = if patterns.is_empty() {
            &self.include_patterns
        } else {
            patterns
        };

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path().to_path_buf();
            if !path.is_file() {
                continue;
            }

            let rel = path.strip_prefix(root).unwrap_or(&path);
            let rel_str = rel.to_string_lossy().replace('\\', "/");

            if self.is_excluded(&rel_str) {
                continue;
            }

            if !self.matches_patterns(&rel_str, effective_patterns) {
                continue;
            }

            let metadata = std::fs::metadata(&path)
                .map_err(|e| HummingbirdError::Context(format!("metadata error: {e}")))?;
            let size = metadata.len() as usize;

            if size > self.max_file_size {
                eprintln!(
                    "WARN: skipping {} — exceeds {} bytes",
                    rel_str, self.max_file_size
                );
                bundle.skipped.push(path);
                continue;
            }

            let raw = std::fs::read(&path).map_err(HummingbirdError::Io)?;

            if Self::is_binary(&raw) {
                bundle.skipped.push(path);
                continue;
            }

            let content = String::from_utf8_lossy(&raw).into_owned();
            bundle.total_bytes += size;
            bundle.files.push(FileEntry {
                path,
                content,
                size_bytes: size,
            });
        }

        Ok(bundle)
    }

    fn is_excluded(&self, rel: &str) -> bool {
        self.exclude_patterns
            .iter()
            .any(|p| Self::glob_match(p, rel))
    }

    fn matches_patterns(&self, rel: &str, patterns: &[String]) -> bool {
        patterns.is_empty() || patterns.iter().any(|p| Self::glob_match(p, rel))
    }

    fn is_binary(data: &[u8]) -> bool {
        let check_len = data.len().min(8192);
        data[..check_len].contains(&0u8)
    }

    fn glob_match(pattern: &str, path: &str) -> bool {
        let pat = glob::Pattern::new(pattern).unwrap_or_else(|_| glob::Pattern::new("**").unwrap());
        pat.matches(path) || pat.matches(&format!("./{path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_tmp() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn gathers_matching_files() {
        let dir = make_tmp();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn foo() {}").unwrap();

        let g = ContextGatherer::default();
        let bundle = g.gather(dir.path(), &["**/*.rs".to_string()]).unwrap();
        assert_eq!(bundle.files.len(), 2);
    }

    #[test]
    fn returns_empty_for_no_matches() {
        let dir = make_tmp();
        fs::write(dir.path().join("readme.md"), "# Hello").unwrap();

        let g = ContextGatherer::default();
        let bundle = g.gather(dir.path(), &["**/*.rs".to_string()]).unwrap();
        assert_eq!(bundle.files.len(), 0);
    }

    #[test]
    fn skips_binary_files() {
        let dir = make_tmp();
        let binary = vec![0u8, 1, 2, 3, 0, 255];
        fs::write(dir.path().join("binary.bin"), &binary).unwrap();

        let g = ContextGatherer::default();
        let bundle = g.gather(dir.path(), &["**/*".to_string()]).unwrap();
        assert_eq!(bundle.files.len(), 0);
        assert_eq!(bundle.skipped.len(), 1);
    }

    #[test]
    fn skips_oversized_files() {
        let dir = make_tmp();
        let content = "x".repeat(200);
        fs::write(dir.path().join("big.txt"), &content).unwrap();

        let g = ContextGatherer::new(100, vec!["**/*".to_string()], vec![]);
        let bundle = g.gather(dir.path(), &[]).unwrap();
        assert_eq!(bundle.files.len(), 0);
        assert_eq!(bundle.skipped.len(), 1);
    }

    #[test]
    fn gathers_nested_directories() {
        let dir = make_tmp();
        let sub = dir.path().join("src").join("utils");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("helper.rs"), "pub fn help() {}").unwrap();

        let g = ContextGatherer::default();
        let bundle = g.gather(dir.path(), &["**/*.rs".to_string()]).unwrap();
        assert_eq!(bundle.files.len(), 1);
        assert!(bundle.files[0].content.contains("help"));
    }
}
