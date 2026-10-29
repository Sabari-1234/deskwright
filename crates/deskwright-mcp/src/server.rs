use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool as McpTool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};

use crate::protocol_mapping::{to_call_tool_result, to_mcp_tool};
use crate::tool::ToolDispatcher;
use crate::tool_registry::ToolRegistry;

const SERVER_NAME: &str = "deskwright";
const SERVER_INSTRUCTIONS: &str = "Drive desktop apps through their accessibility tree, \
    like Playwright for desktop apps. Fastest: act with locators (role/title/value in app) instead of \
    refs, and send a whole known flow (desk_launch_app, clicks, typing, desk_press_keys) as \
    one desk_batch; locators wait for their element, so no delays are needed. Use desk_find \
    or desk_snapshot only to discover what is on screen, desk_read to check values, and \
    desk_screenshot only for visual content. Refs come from the latest snapshot or find and \
    stop working after the next one.";

/// Adapts the [`ToolRegistry`] to `rmcp`'s server handler (Adapter pattern).
pub struct DeskwrightMcpServer {
    registry: Arc<ToolRegistry>,
}

impl DeskwrightMcpServer {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }
}

impl ServerHandler for DeskwrightMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(SERVER_NAME, env!("CARGO_PKG_VERSION")))
            .with_instructions(SERVER_INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = self.registry.specs().into_iter().map(to_mcp_tool).collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        if !self.registry.contains(&request.name) {
            return Err(McpError::invalid_params(
                format!("unknown tool '{}'", request.name),
                None,
            ));
        }
        let output = self
            .registry
            .dispatch(&request.name, request.arguments.unwrap_or_default())
            .await;
        Ok(to_call_tool_result(output).into())
    }

    fn get_tool(&self, name: &str) -> Option<McpTool> {
        self.registry.spec(name).map(to_mcp_tool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::BatchTool;

    fn server_with_batch_tool() -> DeskwrightMcpServer {
        let mut registry = ToolRegistry::new();
        registry.register(BatchTool).expect("one tool");
        DeskwrightMcpServer::new(Arc::new(registry))
    }

    #[test]
    fn advertises_tools_capability_name_and_instructions() {
        let info = server_with_batch_tool().get_info();

        assert!(info.capabilities.tools.is_some());
        assert_eq!(info.server_info.name, SERVER_NAME);
        assert_eq!(info.instructions.as_deref(), Some(SERVER_INSTRUCTIONS));
    }

    #[test]
    fn describes_registered_tools_only() {
        let server = server_with_batch_tool();

        assert_eq!(
            server.get_tool("desk_batch").map(|tool| tool.name),
            Some("desk_batch".into())
        );
        assert!(server.get_tool("desk_click").is_none());
    }
}
