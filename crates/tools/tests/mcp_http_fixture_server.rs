//! Verifies the real HTTP (streamable-http) MCP transport end to end against a
//! minimal MCP server fixture spawned *inside this test* — a real `rmcp`-based
//! server bound to a local TCP port, not a mock and not a dependency on any
//! third-party server's uptime. This exercises the same
//! `McpTransportConfig::Http` code path `mcp_real_server.rs` exercises for
//! stdio, closing the "no real remote HTTP MCP test" gap without relying on
//! external network access.

use std::sync::Arc;

use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as ConnBuilder;
use hyper_util::service::TowerToHyperService;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, ListToolsResult, PaginatedRequestParams,
    ServerInfo, Tool as McpToolDef,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};
use tokio::net::TcpListener;

use tools::{McpServerConfig, McpToolBridge, McpTransportConfig, Tool};

#[derive(Clone, Default)]
struct EchoServer;

impl ServerHandler for EchoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::default()
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let input_schema = serde_json::from_value(serde_json::json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"],
        }))
        .expect("valid JSON schema object");
        Ok(ListToolsResult::with_all_items(vec![McpToolDef::new(
            "echo",
            "Echoes the given text back",
            Arc::new(input_schema),
        )]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let text = request
            .arguments
            .as_ref()
            .and_then(|args| args.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(CallToolResult::success(vec![ContentBlock::text(format!(
            "echo: {text}"
        ))]))
    }
}

/// Spawns the real streamable-HTTP MCP server fixture on an OS-assigned local
/// port and returns its base URL. The accept loop runs for the lifetime of the
/// test process (there is no explicit shutdown — the OS reclaims the port when
/// the test process exits, matching the extension-registry ETag tests' fixture
/// server pattern).
async fn spawn_fixture_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let config = StreamableHttpServerConfig::default()
        .with_stateful_mode(false)
        .with_json_response(true)
        .disable_allowed_hosts();

    let service: StreamableHttpService<EchoServer, LocalSessionManager> =
        StreamableHttpService::new(
            || Ok(EchoServer),
            Arc::new(LocalSessionManager::default()),
            config,
        );

    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let io = TokioIo::new(stream);
            let hyper_service = TowerToHyperService::new(service.clone());
            tokio::spawn(async move {
                let _ = ConnBuilder::new(TokioExecutor::new())
                    .serve_connection(io, hyper_service)
                    .await;
            });
        }
    });

    format!("http://127.0.0.1:{port}/mcp")
}

#[tokio::test]
async fn connects_lists_tools_and_calls_a_real_tool_over_real_streamable_http() {
    let base_url = spawn_fixture_server().await;

    let config = McpServerConfig {
        name: "fixture".to_string(),
        transport: McpTransportConfig::Http {
            url: base_url,
            bearer_token: None,
        },
    };

    let (bridges, descriptors, handle) = McpToolBridge::connect(&config).await.unwrap();

    assert_eq!(descriptors.len(), 1);
    assert_eq!(descriptors[0].name, "echo");

    let echo_tool = bridges
        .iter()
        .find(|b| b.name() == "mcp::fixture::echo")
        .expect("echo tool should be registered under its qualified name");

    let result = echo_tool
        .call(serde_json::json!({ "text": "hello over real http" }))
        .await
        .unwrap();

    assert!(result.to_string().contains("echo: hello over real http"));

    handle.disconnect();
}
