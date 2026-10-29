use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::model::web_address::WebAddress;
use deskwright_core::services::app_launch_service::AppLaunchService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct LaunchAppArguments {
    /// App name or bundle ID, e.g. "Google Chrome" or "com.apple.TextEdit".
    app: String,
    /// Optional http or https address to open in the app, e.g. a search results page.
    url: Option<String>,
}

/// Launches or activates an app so its menus, windows and keyboard input are available.
pub struct LaunchAppTool {
    launches: Arc<AppLaunchService>,
}

impl LaunchAppTool {
    pub fn new(launches: Arc<AppLaunchService>) -> Self {
        Self { launches }
    }
}

#[async_trait]
impl TypedTool for LaunchAppTool {
    type Arguments = LaunchAppArguments;

    const NAME: &'static str = "desk_launch_app";
    const DESCRIPTION: &'static str = "Launch an app, or bring it to the front if it is already \
        running, and wait until it is frontmost. Optionally open an http(s) address in it. Use \
        this before desk_press_keys, and before using the menus of a background app.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(&self, arguments: LaunchAppArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let address = match arguments.url.as_deref().map(WebAddress::parse).transpose() {
            Ok(address) => address,
            Err(invalid_address) => return invalid_address.into(),
        };
        let launches = self.launches.clone();
        run_blocking(move || launches.open(&arguments.app, address.as_ref()))
            .await
            .map(|app| ToolOutput::json(&app))
            .into()
    }
}
