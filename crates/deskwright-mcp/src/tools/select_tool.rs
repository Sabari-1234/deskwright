use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::services::element_locator_service::ElementLocatorService;
use deskwright_core::services::option_service::OptionService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_target_arguments::ElementTargetArguments;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct SelectArguments {
    #[serde(flatten)]
    target: ElementTargetArguments,
    /// The title of the option to choose, ignoring case.
    option: String,
}

/// Chooses an option from a pop-up or menu button.
pub struct SelectTool {
    locator: Arc<ElementLocatorService>,
    options: Arc<OptionService>,
}

impl SelectTool {
    pub fn new(locator: Arc<ElementLocatorService>, options: Arc<OptionService>) -> Self {
        Self { locator, options }
    }
}

#[async_trait]
impl TypedTool for SelectTool {
    type Arguments = SelectArguments;

    const NAME: &'static str = "desk_select";
    const DESCRIPTION: &'static str = "Choose `option` (by title) from a pop-up button or menu \
        button given by `ref` or locator: opens it, waits for the menu, and picks the item. \
        Lists the available options when there is no such option.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(&self, arguments: SelectArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let target = match arguments.target.to_required_target() {
            Ok(target) => target,
            Err(invalid_target) => return invalid_target.into(),
        };
        let locator = self.locator.clone();
        let options = self.options.clone();
        let option = arguments.option;
        let app = arguments.target.app_selector();
        run_blocking(move || {
            let element_ref = locator.resolve(&target)?;
            options.select(&element_ref, &app, &option)?;
            Ok(ToolOutput::text(format!(
                "Selected \"{option}\" in {element_ref}."
            )))
        })
        .await
        .into()
    }
}
