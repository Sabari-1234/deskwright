use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::services::element_locator_service::ElementLocatorService;
use deskwright_core::services::interaction::click_service::{ClickKind, ClickService};
use deskwright_core::{DeskwrightError, DeskwrightResult};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_target_arguments::ElementTargetArguments;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClickArguments {
    #[serde(flatten)]
    target: ElementTargetArguments,
    /// Mouse button: "left" (default) or "right" (opens the context menu).
    #[serde(default)]
    button: MouseButton,
    /// 1 (default) or 2 for a double-click.
    click_count: Option<u8>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    #[default]
    Left,
    Right,
}

impl ClickArguments {
    fn click_kind(&self) -> DeskwrightResult<ClickKind> {
        match (self.button, self.click_count.unwrap_or(1)) {
            (MouseButton::Left, 1) => Ok(ClickKind::Primary),
            (MouseButton::Left, 2) => Ok(ClickKind::Double),
            (MouseButton::Right, 1) => Ok(ClickKind::Secondary),
            (MouseButton::Right, _) => Err(DeskwrightError::InvalidInput(
                "a right-click can only be a single click".to_owned(),
            )),
            (MouseButton::Left, _) => Err(DeskwrightError::InvalidInput(
                "clickCount must be 1 or 2".to_owned(),
            )),
        }
    }
}

/// Clicks, double-clicks or right-clicks an element, falling back from accessibility actions
/// to pointer input.
pub struct ClickTool {
    locator: Arc<ElementLocatorService>,
    clicks: Arc<ClickService>,
}

impl ClickTool {
    pub fn new(locator: Arc<ElementLocatorService>, clicks: Arc<ClickService>) -> Self {
        Self { locator, clicks }
    }
}

#[async_trait]
impl TypedTool for ClickTool {
    type Arguments = ClickArguments;

    const NAME: &'static str = "desk_click";
    const DESCRIPTION: &'static str = "Click an element, given by `ref` or by a locator \
        (role/title/value in `app`, `nth` match), which waits for the element to appear. Uses \
        the element's accessibility press action, or a mouse click at its center when the \
        element doesn't support pressing. `button: \"right\"` opens the context menu; \
        `clickCount: 2` double-clicks (always with the mouse).";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(&self, arguments: ClickArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let (target, kind) = match arguments
            .target
            .to_required_target()
            .and_then(|target| Ok((target, arguments.click_kind()?)))
        {
            Ok(parsed) => parsed,
            Err(invalid_arguments) => return invalid_arguments.into(),
        };
        let locator = self.locator.clone();
        let clicks = self.clicks.clone();
        run_blocking(move || {
            let element_ref = locator.resolve(&target)?;
            let outcome = clicks.click_with(&element_ref, kind)?;
            Ok((element_ref, outcome))
        })
        .await
        .map(|(element_ref, outcome)| {
            ToolOutput::text(format!(
                "Clicked {element_ref} using {}.",
                outcome.strategy_name
            ))
        })
        .into()
    }
}
