use deskwright::Deskwright;
use deskwright_mcp::server::DeskwrightMcpServer;
use rmcp::ServiceExt;

/// Serves every tool over MCP on stdin/stdout until the client disconnects.
pub async fn serve(deskwright: &Deskwright, is_script_enabled: bool) -> anyhow::Result<()> {
    tracing::info!(
        tool_count = deskwright.tools().len(),
        is_script_enabled,
        "deskwright is ready"
    );
    let running_server = DeskwrightMcpServer::new(deskwright.dispatcher())
        .serve(rmcp::transport::stdio())
        .await?;
    running_server.waiting().await?;
    Ok(())
}
