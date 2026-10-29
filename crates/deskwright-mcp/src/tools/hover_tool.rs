use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::services::element_locator_service::ElementLocatorService;
use deskwright_core::services::interaction::pointer_gesture_service::PointerGestureService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_target_arguments::ElementTargetArguments;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct HoverArguments {
    #[serde(flatten)]
    target: ElementTargetArguments,
}

/// Moves the mouse pointer over an element.
pub struct HoverTool {
    locator: Arc<ElementLocatorService>,
    gestures: Arc<PointerGestureService>,
}

impl HoverTool {
    pub fn new(locator: Arc<ElementLocatorService>, gestures: Arc<PointerGestureService>) -> Self {
        Self { locator, gestures }
    }
}

#[async_trait]
impl TypedTool for HoverTool {
    type Arguments = HoverArguments;

    const NAME: &'static str = "desk_hover";
    const DESCRIPTION: &'static str = "Move the mouse pointer over an element (by `ref` or \
        locator) to reveal tooltips, hover buttons or menus. Refused when another app's window \
        covers the element.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(&self, arguments: HoverArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let target = match arguments.target.to_required_target() {
            Ok(target) => target,
            Err(invalid_target) => return invalid_target.into(),
        };
        let locator = self.locator.clone();
        let gestures = self.gestures.clone();
        run_blocking(move || {
            let element_ref = locator.resolve(&target)?;
            gestures.hover(&element_ref)?;
            Ok(ToolOutput::text(format!("Hovering over {element_ref}.")))
        })
        .await
        .into()
    }
}
