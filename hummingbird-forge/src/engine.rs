use hummingbird_common::{HummingbirdError, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct EditSpec {
    pub path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub new_content: String,
}

#[derive(Debug)]
pub struct EditResult {
    pub unified_diff: String,
    pub backup_path: String,
}

pub struct ForgeEngine {
    pub workspace_root: String,
}

impl ForgeEngine {
    pub fn new(workspace_root: impl Into<String>) -> Self {
        Self { workspace_root: workspace_root.into() }
    }

    pub fn apply_edit(&self, spec: &EditSpec) -> Result<EditResult> {
        let path = Path::new(&self.workspace_root).join(&spec.path);
        let original = if path.exists() {
            std::fs::read_to_string(&path).map_err(HummingbirdError::Io)?
        } else {
            String::new()
        };

        let mut lines: Vec<&str> = original.lines().collect();

        // Validate line range
        let start = spec.line_start.saturating_sub(1);
        let end = spec.line_end.min(lines.len());
        if start > lines.len() {
            return Err(HummingbirdError::Tool(format!(
                "line_start {} exceeds file length {}", spec.line_start, lines.len()
            )));
        }

        // Create backup
        let backup_path = format!("{}.bak", path.display());
        if !original.is_empty() {
            std::fs::write(&backup_path, &original).map_err(HummingbirdError::Io)?;
        }

        // Build diff
        let removed: Vec<&str> = lines[start..end].to_vec();
        let added: Vec<&str> = spec.new_content.lines().collect();
        let unified_diff = build_unified_diff(&spec.path, &removed, &added, start);

        // Apply edit
        let new_lines: Vec<&str> = spec.new_content.lines().collect();
        lines.splice(start..end, new_lines.iter().copied());

        let content = if path.exists() && original.ends_with('\n') {
            format!("{}\n", lines.join("\n"))
        } else {
            lines.join("\n")
        };

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(HummingbirdError::Io)?;
            }
        }
        std::fs::write(&path, content).map_err(HummingbirdError::Io)?;

        Ok(EditResult { unified_diff, backup_path })
    }

    pub fn apply_edits(&self, mut specs: Vec<EditSpec>) -> Result<Vec<EditResult>> {
        // Apply in reverse line order to preserve line numbers for earlier edits
        specs.sort_by(|a, b| b.line_start.cmp(&a.line_start));
        self.check_no_overlaps(&specs)?;
        specs.iter().map(|s| self.apply_edit(s)).collect()
    }

    pub fn undo(&self, path: &str) -> Result<()> {
        let backup = format!("{}/{path}.bak", self.workspace_root);
        let target = format!("{}/{path}", self.workspace_root);
        if !Path::new(&backup).exists() {
            return Err(HummingbirdError::Tool(format!("No backup found for '{path}'")));
        }
        std::fs::copy(&backup, &target).map_err(HummingbirdError::Io)?;
        std::fs::remove_file(&backup).map_err(HummingbirdError::Io)?;
        Ok(())
    }

    fn check_no_overlaps(&self, specs: &[EditSpec]) -> Result<()> {
        for i in 0..specs.len() {
            for j in (i + 1)..specs.len() {
                if specs[i].path == specs[j].path {
                    let a = &specs[i];
                    let b = &specs[j];
                    if a.line_start < b.line_end && b.line_start < a.line_end {
                        return Err(HummingbirdError::Tool(format!(
                            "Overlapping edits at lines {}-{} and {}-{}",
                            a.line_start, a.line_end, b.line_start, b.line_end
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

fn build_unified_diff(path: &str, removed: &[&str], added: &[&str], start: usize) -> String {
    let mut out = format!("--- a/{path}\n+++ b/{path}\n@@ -{},{} +{},{} @@\n",
        start + 1, removed.len(), start + 1, added.len());
    for line in removed { out.push_str(&format!("-{line}\n")); }
    for line in added   { out.push_str(&format!("+{line}\n")); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn engine(dir: &TempDir) -> ForgeEngine {
        ForgeEngine::new(dir.path().to_str().unwrap())
    }

    #[test]
    fn applies_single_edit() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("src.rs"), "line1\nline2\nline3\n").unwrap();
        let result = engine(&dir).apply_edit(&EditSpec {
            path: "src.rs".into(),
            line_start: 2,
            line_end: 2,
            new_content: "REPLACED".into(),
        }).unwrap();
        let content = std::fs::read_to_string(dir.path().join("src.rs")).unwrap();
        assert!(content.contains("REPLACED"));
        assert!(!result.unified_diff.is_empty());
    }

    #[test]
    fn creates_backup_before_modification() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("f.rs"), "original\n").unwrap();
        engine(&dir).apply_edit(&EditSpec {
            path: "f.rs".into(), line_start: 1, line_end: 1, new_content: "new".into(),
        }).unwrap();
        assert!(dir.path().join("f.rs.bak").exists() || std::path::Path::new(&format!("{}/f.rs.bak", dir.path().display())).exists());
    }

    #[test]
    fn detects_overlapping_edits() {
        let dir = TempDir::new().unwrap();
        let err = engine(&dir).apply_edits(vec![
            EditSpec { path: "x.rs".into(), line_start: 1, line_end: 5, new_content: "a".into() },
            EditSpec { path: "x.rs".into(), line_start: 3, line_end: 7, new_content: "b".into() },
        ]);
        assert!(err.is_err());
    }

    #[test]
    fn undo_fails_without_backup() {
        let dir = TempDir::new().unwrap();
        let err = engine(&dir).undo("nofile.rs");
        assert!(err.is_err());
    }

    #[test]
    fn creates_new_file_when_missing() {
        let dir = TempDir::new().unwrap();
        engine(&dir).apply_edit(&EditSpec {
            path: "new.rs".into(), line_start: 1, line_end: 1, new_content: "fn new() {}".into(),
        }).unwrap();
        assert!(dir.path().join("new.rs").exists());
    }
}
