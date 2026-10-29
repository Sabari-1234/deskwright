use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::services::app_quit_service::AppQuitService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct QuitAppArguments {
    /// App name or bundle ID. Required: quitting never targets "the frontmost app".
    app: String,
    /// End the app at once, discarding unsaved changes. Default false.
    #[serde(default)]
    force: bool,
}

/// Quits an app and waits until it has exited.
pub struct QuitAppTool {
    quits: Arc<AppQuitService>,
}

impl QuitAppTool {
    pub fn new(quits: Arc<AppQuitService>) -> Self {
        Self { quits }
    }
}

#[async_trait]
impl TypedTool for QuitAppTool {
    type Arguments = QuitAppArguments;

    const NAME: &'static str = "desk_quit_app";
    const DESCRIPTION: &'static str = "Quit an app as its Quit menu item would, and wait until it \
        has exited. Fails if the app is still running, typically because it is asking to save \
        changes. `force: true` ends it at once and LOSES unsaved work; only use it when the \
        user asked.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(&self, arguments: QuitAppArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let quits = self.quits.clone();
        run_blocking(move || quits.quit(&arguments.app, arguments.force))
            .await
            .map(|app| ToolOutput::text(format!("{} has quit.", app.name)))
            .into()
    }
}
