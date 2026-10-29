use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::ports::AppDirectory;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct ListAppsArguments {}

/// Lists running apps. Reads the app directory port directly: listing is the whole use case.
pub struct ListAppsTool {
    apps: Arc<dyn AppDirectory>,
}

impl ListAppsTool {
    pub fn new(apps: Arc<dyn AppDirectory>) -> Self {
        Self { apps }
    }
}

#[async_trait]
impl TypedTool for ListAppsTool {
    type Arguments = ListAppsArguments;

    const NAME: &'static str = "desk_list_apps";
    const DESCRIPTION: &'static str = "List running apps with their names, bundle IDs, \
        process IDs and which one is frontmost.";
    const IS_READ_ONLY: bool = true;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(&self, _arguments: ListAppsArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let apps = self.apps.clone();
        run_blocking(move || apps.running_apps())
            .await
            .map(|running_apps| ToolOutput::json(&running_apps))
            .into()
    }
}
