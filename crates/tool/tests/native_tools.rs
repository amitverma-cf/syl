use std::sync::atomic::{AtomicU64, Ordering};

use tool::{ReadFileTool, RunCommandTool, Tool, ToolError, WriteFileTool};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_workspace() -> std::path::PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("syl-tool-test-{}-{unique}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test]
async fn write_then_read_round_trips_content() {
    let workspace_root = temp_workspace();
    let write_tool = WriteFileTool {
        workspace_root: workspace_root.clone(),
    };
    let read_tool = ReadFileTool {
        workspace_root: workspace_root.clone(),
    };

    write_tool
        .call(serde_json::json!({"path": "notes.txt", "content": "hello world"}))
        .await
        .unwrap();

    let result = read_tool
        .call(serde_json::json!({"path": "notes.txt"}))
        .await
        .unwrap();

    assert_eq!(result["content"], "hello world");
    std::fs::remove_dir_all(&workspace_root).unwrap();
}

#[tokio::test]
async fn write_creates_parent_directories() {
    let workspace_root = temp_workspace();
    let write_tool = WriteFileTool {
        workspace_root: workspace_root.clone(),
    };

    write_tool
        .call(serde_json::json!({"path": "nested/dir/file.txt", "content": "x"}))
        .await
        .unwrap();

    assert!(workspace_root.join("nested/dir/file.txt").exists());
    std::fs::remove_dir_all(&workspace_root).unwrap();
}

#[tokio::test]
async fn read_rejects_absolute_paths() {
    let workspace_root = temp_workspace();
    let read_tool = ReadFileTool {
        workspace_root: workspace_root.clone(),
    };

    let err = read_tool
        .call(serde_json::json!({"path": "/etc/passwd"}))
        .await
        .unwrap_err();

    assert!(matches!(err, ToolError::InvalidArgs(_)));
    std::fs::remove_dir_all(&workspace_root).unwrap();
}

#[tokio::test]
async fn read_rejects_path_traversal_escaping_the_workspace() {
    let workspace_root = temp_workspace();
    let read_tool = ReadFileTool {
        workspace_root: workspace_root.clone(),
    };

    let err = read_tool
        .call(serde_json::json!({"path": "../../../etc/passwd"}))
        .await
        .unwrap_err();

    assert!(matches!(err, ToolError::InvalidArgs(_)));
    std::fs::remove_dir_all(&workspace_root).unwrap();
}

#[tokio::test]
async fn read_missing_file_returns_io_error() {
    let workspace_root = temp_workspace();
    let read_tool = ReadFileTool {
        workspace_root: workspace_root.clone(),
    };

    let err = read_tool
        .call(serde_json::json!({"path": "does-not-exist.txt"}))
        .await
        .unwrap_err();

    assert!(matches!(err, ToolError::Io(_)));
    std::fs::remove_dir_all(&workspace_root).unwrap();
}

#[tokio::test]
async fn run_command_captures_stdout_and_exit_code() {
    let workspace_root = temp_workspace();
    let run_tool = RunCommandTool {
        workspace_root: workspace_root.clone(),
    };

    let result = run_tool
        .call(serde_json::json!({"command": "echo hello"}))
        .await
        .unwrap();

    assert!(result["stdout"].as_str().unwrap().contains("hello"));
    assert_eq!(result["exit_code"], 0);
    std::fs::remove_dir_all(&workspace_root).unwrap();
}

#[tokio::test]
async fn read_rejects_a_symlinked_subdirectory_escaping_the_workspace() {
    let workspace_root = temp_workspace();
    let outside = temp_workspace();
    std::fs::write(outside.join("secret.txt"), "top secret").unwrap();

    let link = workspace_root.join("escape");
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_dir(&outside, &link).is_ok();
    #[cfg(not(windows))]
    let created = std::os::unix::fs::symlink(&outside, &link).is_ok();

    if !created {
        // Creating a symlink needs Developer Mode/admin on Windows or appropriate
        // permissions elsewhere — skip rather than fail CI environments that lack it.
        std::fs::remove_dir_all(&workspace_root).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
        eprintln!("skipping: could not create a symlink in this environment");
        return;
    }

    let read_tool = ReadFileTool {
        workspace_root: workspace_root.clone(),
    };
    let err = read_tool
        .call(serde_json::json!({"path": "escape/secret.txt"}))
        .await
        .unwrap_err();

    assert!(matches!(err, ToolError::InvalidArgs(_)));
    std::fs::remove_dir_all(&outside).unwrap();
    let _ = std::fs::remove_dir_all(&workspace_root);
}

#[tokio::test]
async fn run_command_runs_in_the_workspace_root() {
    let workspace_root = temp_workspace();
    std::fs::write(workspace_root.join("marker.txt"), "x").unwrap();
    let run_tool = RunCommandTool {
        workspace_root: workspace_root.clone(),
    };

    let list_command = if cfg!(target_os = "windows") {
        "dir /b"
    } else {
        "ls"
    };

    let result = run_tool
        .call(serde_json::json!({"command": list_command}))
        .await
        .unwrap();

    assert!(result["stdout"].as_str().unwrap().contains("marker.txt"));
    std::fs::remove_dir_all(&workspace_root).unwrap();
}
