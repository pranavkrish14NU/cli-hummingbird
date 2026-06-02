use hummingbird_common::{HummingbirdError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::history::MessageHistory;

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

const SESSIONS_DIR: &str = ".hummingbird/sessions";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub summary: String,
    pub history: MessageHistory,
}

impl Session {
    pub fn new(summary: impl Into<String>) -> Self {
        let now = unix_now();
        Self {
            id: unique_id(),
            created_at: now,
            updated_at: now,
            summary: summary.into(),
            history: MessageHistory::new(),
        }
    }

    pub fn save(&self, workspace: &Path) -> Result<PathBuf> {
        let dir = workspace.join(SESSIONS_DIR);
        std::fs::create_dir_all(&dir).map_err(HummingbirdError::Io)?;
        let path = dir.join(format!("{}.json", self.id));
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json).map_err(HummingbirdError::Io)?;
        Ok(path)
    }

    pub fn load(workspace: &Path, id: &str) -> Result<Self> {
        let path = workspace.join(SESSIONS_DIR).join(format!("{id}.json"));
        let json = std::fs::read_to_string(&path).map_err(HummingbirdError::Io)?;
        serde_json::from_str(&json).map_err(Into::into)
    }

    pub fn list(workspace: &Path) -> Result<Vec<SessionMeta>> {
        let dir = workspace.join(SESSIONS_DIR);
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut metas = vec![];
        for entry in std::fs::read_dir(&dir).map_err(HummingbirdError::Io)? {
            let entry = entry.map_err(HummingbirdError::Io)?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(s) = serde_json::from_str::<Session>(&json) {
                        metas.push(SessionMeta {
                            id: s.id,
                            created_at: s.created_at,
                            summary: s.summary,
                            message_count: s.history.len(),
                        });
                    }
                }
            }
        }
        metas.sort_by_key(|m| std::cmp::Reverse(m.created_at));
        Ok(metas)
    }

    pub fn prune(workspace: &Path, older_than_ms: u64) -> Result<usize> {
        let dir = workspace.join(SESSIONS_DIR);
        if !dir.exists() {
            return Ok(0);
        }
        let cutoff = unix_now().saturating_sub(older_than_ms);
        let mut pruned = 0;
        for entry in std::fs::read_dir(&dir).map_err(HummingbirdError::Io)? {
            let entry = entry.map_err(HummingbirdError::Io)?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(s) = serde_json::from_str::<Session>(&json) {
                        if s.created_at < cutoff {
                            let _ = std::fs::remove_file(&path);
                            pruned += 1;
                        }
                    }
                }
            }
        }
        Ok(pruned)
    }
}

#[derive(Debug, Clone)]
pub struct SessionMeta {
    pub id: String,
    pub created_at: u64,
    pub summary: String,
    pub message_count: usize,
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn unique_id() -> String {
    let ts = unix_now();
    let seq = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{ts:016x}{seq:04x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn create_save_load_session() {
        let dir = TempDir::new().unwrap();
        let mut session = Session::new("Test session");
        session.history.push_user("Hello");
        session.save(dir.path()).unwrap();
        let loaded = Session::load(dir.path(), &session.id).unwrap();
        assert_eq!(loaded.summary, "Test session");
        assert_eq!(loaded.history.len(), 1);
    }

    #[test]
    fn list_sessions() {
        let dir = TempDir::new().unwrap();
        Session::new("Session A").save(dir.path()).unwrap();
        Session::new("Session B").save(dir.path()).unwrap();
        let list = Session::list(dir.path()).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn list_returns_empty_when_no_dir() {
        let dir = TempDir::new().unwrap();
        let list = Session::list(dir.path()).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn prune_old_sessions() {
        let dir = TempDir::new().unwrap();
        Session::new("Old").save(dir.path()).unwrap();
        // Sleep so the session's created_at is strictly before unix_now() at prune time
        std::thread::sleep(std::time::Duration::from_millis(50));
        // Prune sessions older than 1ms — everything saved >50ms ago qualifies
        let pruned = Session::prune(dir.path(), 1).unwrap();
        assert_eq!(pruned, 1);
        assert!(Session::list(dir.path()).unwrap().is_empty());
    }
}
