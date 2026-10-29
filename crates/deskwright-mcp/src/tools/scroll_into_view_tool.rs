use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::services::element_locator_service::ElementLocatorService;
use deskwright_core::services::interaction::scroll_service::{ScrollOutcome, ScrollService};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_target_arguments::ElementTargetArguments;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct ScrollIntoViewArguments {
    #[serde(flatten)]
    target: ElementTargetArguments,
}

/// Scrolls an element into view.
pub struct ScrollIntoViewTool {
    locator: Arc<ElementLocatorService>,
    scrolls: Arc<ScrollService>,
}

impl ScrollIntoViewTool {
    pub fn new(locator: Arc<ElementLocatorService>, scrolls: Arc<ScrollService>) -> Self {
        Self { locator, scrolls }
    }
}

#[async_trait]
impl TypedTool for ScrollIntoViewTool {
    type Arguments = ScrollIntoViewArguments;

    const NAME: &'static str = "desk_scroll_into_view";
    const DESCRIPTION: &'static str = "Scroll the element given by `ref` or locator into view \
        inside its scroll areas, e.g. before a pointer click or screenshot of a row far down a \
        list.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(
        &self,
        arguments: ScrollIntoViewArguments,
        _context: &ToolContext<'_>,
    ) -> ToolOutput {
        let target = match arguments.target.to_required_target() {
            Ok(target) => target,
            Err(invalid_target) => return invalid_target.into(),
        };
        let locator = self.locator.clone();
        let scrolls = self.scrolls.clone();
        run_blocking(move || {
            let element_ref = locator.resolve(&target)?;
            let message = match scrolls.scroll_into_view(&element_ref)? {
                ScrollOutcome::Scrolled => format!("Scrolled {element_ref} into view."),
                ScrollOutcome::AlreadyOnScreen => format!(
                    "{element_ref} is already on screen (the app can't scroll it, so nothing \
                     was needed)."
                ),
            };
            Ok(ToolOutput::text(message))
        })
        .await
        .into()
    }
}
