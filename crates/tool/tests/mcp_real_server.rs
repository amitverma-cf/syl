use tool::{McpServerConfig, McpToolBridge, McpTransportConfig, Tool, ToolError};

/// Connects to the official MCP reference filesystem server
/// (`@modelcontextprotocol/server-filesystem`, spawned via `npx`) over real stdio, lists its
/// real tools, and calls its file-read tool end to end against a real file on disk. Requires
/// `npx`/network access to resolve the package on first run. Run manually with
/// `cargo test -p tool --test mcp_real_server -- --ignored`.
#[tokio::test]
#[ignore]
async fn connects_lists_tools_and_reads_a_real_file_via_a_real_mcp_server() {
    let workspace =
        std::env::temp_dir().join(format!("syl-mcp-real-server-test-{}", std::process::id()));
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(
        workspace.join("hello.txt"),
        "hello from the real filesystem MCP server",
    )
    .unwrap();

    // On Windows, `npx` is a `.cmd` shim that `Command::new` won't resolve without going
    // through a shell — same reason `RunCommandTool` wraps user commands in `cmd /C` on
    // Windows. A real MCP config would name the shim directly the same way.
    let npx_command = if cfg!(windows) { "npx.cmd" } else { "npx" };
    let config = McpServerConfig {
        name: "filesystem".to_string(),
        transport: McpTransportConfig::Stdio {
            command: npx_command.to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                workspace.to_string_lossy().to_string(),
            ],
        },
    };

    let (bridges, descriptors, _handle) = McpToolBridge::connect(&config).await.unwrap();

    assert!(
        !bridges.is_empty(),
        "expected the filesystem server to advertise at least one tool"
    );

    let read_tool_name = descriptors
        .iter()
        .find(|d| d.name.contains("read"))
        .map(|d| d.name.clone())
        .expect("filesystem server should expose a read-file tool");

    let read_tool = bridges
        .iter()
        .find(|b| b.name() == format!("mcp::filesystem::{read_tool_name}"))
        .expect("read tool should be registered under its qualified name");

    let result = read_tool
        .call(serde_json::json!({ "path": workspace.join("hello.txt").to_string_lossy() }))
        .await
        .unwrap();

    assert!(result
        .to_string()
        .contains("hello from the real filesystem MCP server"));

    std::fs::remove_dir_all(&workspace).ok();
}

/// Proves `McpConnectionHandle::disconnect` actually tears the real child process down —
/// not just that the in-process tool bridge stops working, which dropping the `Arc`
/// would already prove on its own. Requires `npx`/network access, same as the test above.
/// Run manually with `cargo test -p tool --test mcp_real_server -- --ignored`.
#[tokio::test]
#[ignore]
async fn disconnect_actually_terminates_the_real_child_process() {
    let workspace =
        std::env::temp_dir().join(format!("syl-mcp-disconnect-test-{}", std::process::id()));
    std::fs::create_dir_all(&workspace).unwrap();

    let npx_command = if cfg!(windows) { "npx.cmd" } else { "npx" };
    let config = McpServerConfig {
        name: "filesystem".to_string(),
        transport: McpTransportConfig::Stdio {
            command: npx_command.to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                workspace.to_string_lossy().to_string(),
            ],
        },
    };

    let (bridges, descriptors, handle) = McpToolBridge::connect(&config).await.unwrap();
    assert!(!bridges.is_empty());

    let list_tool_name = descriptors
        .iter()
        .find(|d| d.name.contains("list"))
        .map(|d| d.name.clone())
        .expect("filesystem server should expose a list-directory tool");
    let list_tool = bridges
        .into_iter()
        .find(|b| b.name() == format!("mcp::filesystem::{list_tool_name}"))
        .unwrap();

    // A call works fine while still connected.
    list_tool
        .call(serde_json::json!({ "path": workspace.to_string_lossy() }))
        .await
        .expect("tool call should succeed before disconnect");

    handle.disconnect();
    // Give the async cancellation/cleanup task a moment to actually run and kill the
    // child process (rmcp's ChildWithCleanup spawns the kill as its own tokio task).
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let err = list_tool
        .call(serde_json::json!({ "path": workspace.to_string_lossy() }))
        .await
        .expect_err("tool call must fail once the transport has been explicitly disconnected");
    assert!(matches!(err, ToolError::Execution(_)));

    std::fs::remove_dir_all(&workspace).ok();
}
