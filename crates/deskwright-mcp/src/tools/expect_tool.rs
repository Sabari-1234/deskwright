use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::model::element_condition::{
    ElementCondition, HasCount, HasText, IsChecked, IsEnabled, IsFocused, IsHidden, IsVisible,
};
use deskwright_core::services::expectation_service::ExpectationService;
use deskwright_core::{DeskwrightError, DeskwrightResult};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_target_arguments::ElementTargetArguments;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ExpectedCondition {
    Visible,
    Hidden,
    Enabled,
    Disabled,
    Checked,
    Unchecked,
    Focused,
    /// The value (or title) equals `text`.
    HasText,
    /// The value (or title) contains `text`, ignoring case.
    ContainsText,
    /// Exactly `count` elements match.
    Count,
}

#[derive(Deserialize, JsonSchema)]
pub struct ExpectArguments {
    /// The element(s) to check: a locator (role/title/value/placeholder/identifier, app,
    /// nth or last, timeoutSeconds). Refs are refused, since they can't be re-searched.
    #[serde(flatten)]
    target: ElementTargetArguments,
    /// The state to wait for.
    condition: ExpectedCondition,
    /// For "hasText" and "containsText".
    text: Option<String>,
    /// For "count".
    count: Option<usize>,
}

impl ExpectArguments {
    fn condition(&self) -> DeskwrightResult<Box<dyn ElementCondition>> {
        let condition: Box<dyn ElementCondition> = match self.condition {
            ExpectedCondition::Visible => Box::new(IsVisible),
            ExpectedCondition::Hidden => Box::new(IsHidden),
            ExpectedCondition::Enabled => Box::new(IsEnabled(true)),
            ExpectedCondition::Disabled => Box::new(IsEnabled(false)),
            ExpectedCondition::Checked => Box::new(IsChecked(true)),
            ExpectedCondition::Unchecked => Box::new(IsChecked(false)),
            ExpectedCondition::Focused => Box::new(IsFocused),
            ExpectedCondition::HasText => Box::new(HasText {
                text: self.required_text()?,
                is_exact: true,
            }),
            ExpectedCondition::ContainsText => Box::new(HasText {
                text: self.required_text()?,
                is_exact: false,
            }),
            ExpectedCondition::Count => Box::new(HasCount(self.count.ok_or_else(|| {
                DeskwrightError::InvalidInput("condition \"count\" needs count".to_owned())
            })?)),
        };
        Ok(condition)
    }

    fn required_text(&self) -> DeskwrightResult<String> {
        self.text
            .clone()
            .ok_or_else(|| DeskwrightError::InvalidInput("text conditions need text".to_owned()))
    }
}

/// Asserts a state of the UI, retrying until it holds, like Playwright's `expect(locator)`.
pub struct ExpectTool {
    expectations: Arc<ExpectationService>,
}

impl ExpectTool {
    pub fn new(expectations: Arc<ExpectationService>) -> Self {
        Self { expectations }
    }
}

#[async_trait]
impl TypedTool for ExpectTool {
    type Arguments = ExpectArguments;

    const NAME: &'static str = "desk_expect";
    const DESCRIPTION: &'static str = "Assert that a located element reaches a state, retrying \
        until it does or `timeoutSeconds` (default 5) passes: `condition` is visible, hidden, \
        enabled, disabled, checked, unchecked, focused, hasText / containsText (`text`), or \
        count (`count`). Fails with what was seen instead. Use it in batches to verify each \
        step, e.g. that a message was sent.";
    const IS_READ_ONLY: bool = true;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(&self, arguments: ExpectArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let parsed = arguments
            .target
            .to_required_locator()
            .and_then(|locator| Ok((locator, arguments.condition()?)));
        let (locator, condition) = match parsed {
            Ok(parsed) => parsed,
            Err(invalid_arguments) => return invalid_arguments.into(),
        };
        let expectations = self.expectations.clone();
        run_blocking(move || {
            let selected = expectations.expect(&locator, condition.as_ref())?;
            let element = selected
                .map(|element| format!(" ({})", element.element_ref))
                .unwrap_or_default();
            Ok(ToolOutput::text(format!(
                "Passed: {} {}{element}.",
                locator.query,
                condition.description()
            )))
        })
        .await
        .into()
    }
}
