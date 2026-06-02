use async_trait::async_trait;
use hummingbird_common::{HummingbirdError, Result};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use crate::tool::{Tool, ToolResult};

const DEFAULT_TIMEOUT_SECS: u64 = 60;

static BLOCKED_PREFIXES: &[&str] = &[
    "rm -rf /",
    "rm -rf ~",
    "sudo rm",
    "mkfs",
    "dd if=",
    ":(){:|:&};:",
    "chmod -R 777 /",
    "> /dev/sda",
    "format c:",
];

pub struct ShellExec {
    pub workspace_root: String,
    pub timeout_secs: u64,
    pub blocked_commands: Vec<String>,
}

impl ShellExec {
    pub fn new(workspace_root: impl Into<String>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            blocked_commands: BLOCKED_PREFIXES.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn is_blocked(&self, cmd: &str) -> bool {
        let lower = cmd.trim().to_lowercase();
        BLOCKED_PREFIXES.iter().any(|b| lower.starts_with(b))
            || self.blocked_commands.iter().any(|b| lower.starts_with(b.as_str()))
    }
}

#[async_trait]
impl Tool for ShellExec {
    fn name(&self) -> &str { "shell_exec" }

    fn description(&self) -> &str {
        "Execute a shell command in the workspace directory. Dangerous commands are blocked."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to run" },
                "timeout_secs": { "type": "integer", "description": "Timeout in seconds (default 60)" }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        let command = params["command"].as_str()
            .ok_or_else(|| HummingbirdError::Tool("missing 'command' parameter".into()))?;

        if self.is_blocked(command) {
            return Ok(ToolResult::err(format!("Command blocked by security policy: '{command}'")));
        }

        let timeout_secs = params["timeout_secs"].as_u64().unwrap_or(self.timeout_secs);
        let workspace = Path::new(&self.workspace_root);

        #[cfg(unix)]
        let mut child = Command::new("sh");
        #[cfg(windows)]
        let mut child = Command::new("cmd");

        #[cfg(unix)]
        child.args(["-c", command]);
        #[cfg(windows)]
        child.args(["/C", command]);

        child.current_dir(workspace);

        let fut = child.output();
        match timeout(Duration::from_secs(timeout_secs), fut).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let combined = if stderr.is_empty() {
                    stdout
                } else {
                    format!("{stdout}\nSTDERR:\n{stderr}")
                };
                let exit_code = output.status.code().unwrap_or(-1);
                if output.status.success() {
                    Ok(ToolResult::ok(format!("exit={exit_code}\n{combined}")))
                } else {
                    Ok(ToolResult::err(format!("exit={exit_code}\n{combined}")))
                }
            }
            Ok(Err(e)) => Ok(ToolResult::err(format!("Failed to spawn process: {e}"))),
            Err(_) => Ok(ToolResult::err(format!("Command timed out after {timeout_secs}s"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> ShellExec {
        ShellExec::new(".")
    }

    #[tokio::test]
    async fn blocks_dangerous_commands() {
        let result = shell().execute(json!({"command": "rm -rf /etc"})).await.unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("blocked"));
    }

    #[tokio::test]
    async fn blocks_sudo() {
        let s = ShellExec {
            workspace_root: ".".into(),
            timeout_secs: 5,
            blocked_commands: vec!["sudo".into()],
        };
        let result = s.execute(json!({"command": "sudo rm -rf /"})).await.unwrap();
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn captures_stdout_and_stderr() {
        #[cfg(unix)]
        let result = shell().execute(json!({"command": "echo hello"})).await.unwrap();
        #[cfg(windows)]
        let result = shell().execute(json!({"command": "echo hello"})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn times_out_slow_commands() {
        #[cfg(unix)]
        let result = shell().execute(json!({"command": "sleep 10", "timeout_secs": 1})).await.unwrap();
        #[cfg(windows)]
        let result = shell().execute(json!({"command": "powershell -c Start-Sleep 10", "timeout_secs": 1})).await.unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("timed out"));
    }
}
