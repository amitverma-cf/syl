use tools::{ReadFileTool, RunCommandTool, Tool, WriteFileTool};

fn assert_valid_object_schema(tool: &dyn Tool) {
    let schema = tool.input_schema();
    assert_eq!(
        schema.get("type").and_then(|v| v.as_str()),
        Some("object"),
        "{}'s input_schema must be a JSON Schema object",
        tool.name()
    );
    assert!(
        schema.get("properties").is_some(),
        "{}'s input_schema must declare properties",
        tool.name()
    );
    assert!(
        !tool.description().is_empty(),
        "{} must have a non-empty description",
        tool.name()
    );
}

#[test]
fn every_native_tool_exposes_a_valid_schema_and_description() {
    let workspace_root = std::env::temp_dir();
    assert_valid_object_schema(&ReadFileTool {
        workspace_root: workspace_root.clone(),
    });
    assert_valid_object_schema(&WriteFileTool {
        workspace_root: workspace_root.clone(),
    });
    assert_valid_object_schema(&RunCommandTool { workspace_root });
}

#[test]
fn tool_executor_tool_specs_reflects_registered_tools() {
    use tools::{AlwaysApprove, ToolExecutor};

    struct NoopPermissions;
    impl memory::ToolPermissionStore for NoopPermissions {
        fn get_tool_permission(
            &self,
            _conversation_id: &str,
            _tool_name: &str,
        ) -> Result<Option<memory::ToolPermissionDecision>, memory::MemoryError> {
            Ok(None)
        }
        fn set_tool_permission(
            &self,
            _conversation_id: &str,
            _tool_name: &str,
            _decision: memory::ToolPermissionDecision,
        ) -> Result<(), memory::MemoryError> {
            Ok(())
        }
        fn clear_tool_permission(
            &self,
            _conversation_id: &str,
            _tool_name: &str,
        ) -> Result<(), memory::MemoryError> {
            Ok(())
        }
        fn list_tool_permissions(
            &self,
            _conversation_id: &str,
        ) -> Result<Vec<(String, memory::ToolPermissionDecision)>, memory::MemoryError> {
            Ok(Vec::new())
        }
    }

    let executor = ToolExecutor::new(
        std::sync::Arc::new(AlwaysApprove),
        std::sync::Arc::new(NoopPermissions),
    );
    executor.register(std::sync::Arc::new(ReadFileTool {
        workspace_root: std::env::temp_dir(),
    }));

    let specs = executor.tool_specs();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].name, "read_file");
    assert!(!specs[0].description.is_empty());
    assert_eq!(specs[0].input_schema["type"], "object");
}

#[test]
fn tool_specs_filtered_empty_allowlist_means_unrestricted() {
    use tools::{AlwaysApprove, ToolExecutor};

    struct NoopPermissions;
    impl memory::ToolPermissionStore for NoopPermissions {
        fn get_tool_permission(
            &self,
            _conversation_id: &str,
            _tool_name: &str,
        ) -> Result<Option<memory::ToolPermissionDecision>, memory::MemoryError> {
            Ok(None)
        }
        fn set_tool_permission(
            &self,
            _conversation_id: &str,
            _tool_name: &str,
            _decision: memory::ToolPermissionDecision,
        ) -> Result<(), memory::MemoryError> {
            Ok(())
        }
        fn clear_tool_permission(
            &self,
            _conversation_id: &str,
            _tool_name: &str,
        ) -> Result<(), memory::MemoryError> {
            Ok(())
        }
        fn list_tool_permissions(
            &self,
            _conversation_id: &str,
        ) -> Result<Vec<(String, memory::ToolPermissionDecision)>, memory::MemoryError> {
            Ok(Vec::new())
        }
    }

    let executor = ToolExecutor::new(
        std::sync::Arc::new(AlwaysApprove),
        std::sync::Arc::new(NoopPermissions),
    );
    executor.register(std::sync::Arc::new(ReadFileTool {
        workspace_root: std::env::temp_dir(),
    }));
    executor.register(std::sync::Arc::new(WriteFileTool {
        workspace_root: std::env::temp_dir(),
    }));

    assert_eq!(executor.tool_specs_filtered(&[]).len(), 2);

    let filtered = executor.tool_specs_filtered(&["read_file".to_string()]);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "read_file");

    assert!(executor
        .tool_specs_filtered(&["does_not_exist".to_string()])
        .is_empty());
}
