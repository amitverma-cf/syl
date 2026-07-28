use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::{Permission, Tool, ToolError};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            other => result.push(other.as_os_str()),
        }
    }
    result
}

fn resolve_within_root(root: &Path, rel: &str) -> Result<PathBuf, ToolError> {
    if Path::new(rel).is_absolute() {
        return Err(ToolError::InvalidArgs(format!(
            "path must be relative to the workspace: {rel}"
        )));
    }
    let normalized_root = normalize(root);
    let normalized = normalize(&root.join(rel));
    if !normalized.starts_with(&normalized_root) {
        return Err(ToolError::InvalidArgs(format!(
            "path escapes the workspace root: {rel}"
        )));
    }
    Ok(normalized)
}

fn require_str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::InvalidArgs(format!("missing '{key}' argument")))
}

pub struct ReadFileTool {
    pub workspace_root: PathBuf,
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn required_permission(&self) -> Permission {
        Permission::Ask
    }

    async fn call(&self, args: Value) -> Result<Value, ToolError> {
        let rel_path = require_str_arg(&args, "path")?;
        let resolved = resolve_within_root(&self.workspace_root, rel_path)?;
        let content = tokio::fs::read_to_string(&resolved).await?;
        Ok(json!({ "content": content }))
    }
}

pub struct WriteFileTool {
    pub workspace_root: PathBuf,
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn required_permission(&self) -> Permission {
        Permission::Ask
    }

    async fn call(&self, args: Value) -> Result<Value, ToolError> {
        let rel_path = require_str_arg(&args, "path")?;
        let content = require_str_arg(&args, "content")?;
        let resolved = resolve_within_root(&self.workspace_root, rel_path)?;
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&resolved, content).await?;
        Ok(json!({ "bytes_written": content.len() }))
    }
}

pub struct RunCommandTool {
    pub workspace_root: PathBuf,
}

#[async_trait]
impl Tool for RunCommandTool {
    fn name(&self) -> &str {
        "run_command"
    }

    fn required_permission(&self) -> Permission {
        Permission::Ask
    }

    async fn call(&self, args: Value) -> Result<Value, ToolError> {
        let command = require_str_arg(&args, "command")?;

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = tokio::process::Command::new("cmd");
            c.arg("/C").arg(command);
            c
        } else {
            let mut c = tokio::process::Command::new("sh");
            c.arg("-c").arg(command);
            c
        };
        cmd.current_dir(&self.workspace_root);

        let output = tokio::time::timeout(COMMAND_TIMEOUT, cmd.output())
            .await
            .map_err(|_| ToolError::Execution("command timed out".to_string()))??;

        Ok(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code(),
        }))
    }
}
