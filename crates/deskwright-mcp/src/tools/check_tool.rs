use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::services::element_locator_service::ElementLocatorService;
use deskwright_core::services::interaction::toggle_service::{ToggleOutcome, ToggleService};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_target_arguments::ElementTargetArguments;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct CheckArguments {
    #[serde(flatten)]
    target: ElementTargetArguments,
    /// The state to end in: true to check (default), false to uncheck.
    checked: Option<bool>,
}

/// Puts a checkbox, radio button or switch into a given state.
pub struct CheckTool {
    locator: Arc<ElementLocatorService>,
    toggles: Arc<ToggleService>,
}

impl CheckTool {
    pub fn new(locator: Arc<ElementLocatorService>, toggles: Arc<ToggleService>) -> Self {
        Self { locator, toggles }
    }
}

#[async_trait]
impl TypedTool for CheckTool {
    type Arguments = CheckArguments;

    const NAME: &'static str = "desk_check";
    const DESCRIPTION: &'static str = "Check (or with `checked: false`, uncheck) a checkbox, \
        radio button or switch given by `ref` or locator. Clicks only if the state differs, then \
        verifies the new state, so repeating it is safe.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(&self, arguments: CheckArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let target = match arguments.target.to_required_target() {
            Ok(target) => target,
            Err(invalid_target) => return invalid_target.into(),
        };
        let should_be_checked = arguments.checked.unwrap_or(true);
        let state_name = if should_be_checked {
            "checked"
        } else {
            "unchecked"
        };
        let locator = self.locator.clone();
        let toggles = self.toggles.clone();
        run_blocking(move || {
            let element_ref = locator.resolve(&target)?;
            let message = match toggles.set_checked(&element_ref, should_be_checked)? {
                ToggleOutcome::AlreadyInState => format!("{element_ref} was already {state_name}."),
                ToggleOutcome::Toggled { strategy_name } => {
                    format!("{element_ref} is now {state_name} (using {strategy_name}).")
                }
            };
            Ok(ToolOutput::text(message))
        })
        .await
        .into()
    }
}
