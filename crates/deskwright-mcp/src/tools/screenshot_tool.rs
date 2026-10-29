use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::model::element::ElementRef;
use deskwright_core::services::screenshot_service::{ScreenshotService, ScreenshotTarget};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct ScreenshotArguments {
    /// Element ref to capture. Omit to capture the whole main display.
    #[serde(rename = "ref")]
    element_ref: Option<String>,
}

/// Captures an element or the main display as a PNG.
pub struct ScreenshotTool {
    screenshots: Arc<ScreenshotService>,
}

impl ScreenshotTool {
    pub fn new(screenshots: Arc<ScreenshotService>) -> Self {
        Self { screenshots }
    }
}

#[async_trait]
impl TypedTool for ScreenshotTool {
    type Arguments = ScreenshotArguments;

    const NAME: &'static str = "desk_screenshot";
    const DESCRIPTION: &'static str = "Capture a PNG of one element (by ref) or of the main \
        display. Prefer desk_snapshot for understanding UI; use this for visual content such \
        as images, canvases or games. Needs Screen Recording permission.";
    const IS_READ_ONLY: bool = true;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(&self, arguments: ScreenshotArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let target = match arguments
            .element_ref
            .as_deref()
            .map(ElementRef::parse)
            .transpose()
        {
            Ok(Some(element_ref)) => ScreenshotTarget::Element(element_ref),
            Ok(None) => ScreenshotTarget::MainDisplay,
            Err(invalid_ref) => return invalid_ref.into(),
        };
        let screenshots = self.screenshots.clone();
        run_blocking(move || screenshots.capture(&target))
            .await
            .map(|image| ToolOutput::png(&image))
            .into()
    }
}
