use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::services::element_locator_service::ElementLocatorService;
use deskwright_core::services::element_read_service::ElementReadService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_target_arguments::ElementTargetArguments;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct ReadArguments {
    #[serde(flatten)]
    target: ElementTargetArguments,
}

/// Reads the live state of one element.
pub struct ReadTool {
    locator: Arc<ElementLocatorService>,
    reader: Arc<ElementReadService>,
}

impl ReadTool {
    pub fn new(locator: Arc<ElementLocatorService>, reader: Arc<ElementReadService>) -> Self {
        Self { locator, reader }
    }
}

#[async_trait]
impl TypedTool for ReadTool {
    type Arguments = ReadArguments;

    const NAME: &'static str = "desk_read";
    const DESCRIPTION: &'static str = "Read the current role, title, value, description, \
        state and bounds of one element, given by `ref` or by a locator (role/title/value in \
        `app`, `nth` match), which waits for the element to appear.";
    const IS_READ_ONLY: bool = true;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(&self, arguments: ReadArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let target = match arguments.target.to_required_target() {
            Ok(target) => target,
            Err(invalid_target) => return invalid_target.into(),
        };
        let locator = self.locator.clone();
        let reader = self.reader.clone();
        run_blocking(move || reader.read(&locator.resolve(&target)?))
            .await
            .map(|summary| ToolOutput::json(&summary))
            .into()
    }
}
