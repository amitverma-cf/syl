use tools::{McpServerConfig, McpTransportConfig};

#[test]
fn stdio_config_round_trips_through_json() {
    let config = McpServerConfig {
        name: "filesystem".to_string(),
        transport: McpTransportConfig::Stdio {
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
            ],
        },
    };
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains(r#""transport":"stdio""#));
    assert!(json.contains(r#""command":"npx""#));

    let round_tripped: McpServerConfig = serde_json::from_str(&json).unwrap();
    match round_tripped.transport {
        McpTransportConfig::Stdio { command, args } => {
            assert_eq!(command, "npx");
            assert_eq!(args, vec!["-y", "@modelcontextprotocol/server-filesystem"]);
        }
        McpTransportConfig::Http { .. } => panic!("expected stdio transport"),
    }
}

#[test]
fn http_config_round_trips_through_json_with_bearer_token() {
    let config = McpServerConfig {
        name: "linear".to_string(),
        transport: McpTransportConfig::Http {
            url: "https://mcp.linear.app/sse".to_string(),
            bearer_token: Some("secret-token".to_string()),
        },
    };
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains(r#""transport":"http""#));
    assert!(json.contains("mcp.linear.app"));
    assert!(json.contains("secret-token"));

    let round_tripped: McpServerConfig = serde_json::from_str(&json).unwrap();
    match round_tripped.transport {
        McpTransportConfig::Http { url, bearer_token } => {
            assert_eq!(url, "https://mcp.linear.app/sse");
            assert_eq!(bearer_token.as_deref(), Some("secret-token"));
        }
        McpTransportConfig::Stdio { .. } => panic!("expected http transport"),
    }
}

#[test]
fn http_config_omits_bearer_token_field_when_absent() {
    let config = McpServerConfig {
        name: "public-server".to_string(),
        transport: McpTransportConfig::Http {
            url: "https://example.com/mcp".to_string(),
            bearer_token: None,
        },
    };
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.contains("bearerToken"));
}

#[test]
fn load_mcp_servers_round_trips_a_mixed_list_via_disk() {
    let path =
        std::env::temp_dir().join(format!("syl-mcp-config-test-{}.json", std::process::id()));
    let servers = vec![
        McpServerConfig {
            name: "local".to_string(),
            transport: McpTransportConfig::Stdio {
                command: "npx".to_string(),
                args: vec![],
            },
        },
        McpServerConfig {
            name: "remote".to_string(),
            transport: McpTransportConfig::Http {
                url: "https://example.com/mcp".to_string(),
                bearer_token: None,
            },
        },
    ];
    tools::save_mcp_servers(&path, &servers).unwrap();

    let loaded = tools::load_mcp_servers(&path);
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].name, "local");
    assert_eq!(loaded[1].name, "remote");

    std::fs::remove_file(&path).ok();
}
