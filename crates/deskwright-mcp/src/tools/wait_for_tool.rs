use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::model::app::AppSelector;
use deskwright_core::model::element_condition::IsHidden;
use deskwright_core::model::element_target::{Locator, MatchPosition};
use deskwright_core::services::element_wait_service::ElementWaitService;
use deskwright_core::services::expectation_service::ExpectationService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_outline::render_search_results;
use crate::match_limit::match_limit;
use crate::query_arguments::QueryArguments;
use crate::timeout_argument::timeout_from_seconds;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WaitForArguments {
    #[serde(flatten)]
    query: QueryArguments,
    /// App name or bundle ID. Omit for the frontmost app.
    app: Option<String>,
    /// How long to wait, in seconds (default 5, at most 30).
    timeout_seconds: Option<f64>,
    /// Maximum matches to return (1-200, default 25).
    limit: Option<usize>,
    /// "attached" (default): wait until a match exists. "hidden": wait until no match is
    /// visible, e.g. until a progress indicator or dialog goes away.
    #[serde(default)]
    state: WaitState,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WaitState {
    #[default]
    Attached,
    Hidden,
}

/// Waits for elements to appear or disappear, so clients never have to sleep for a guessed
/// time.
pub struct WaitForTool {
    waits: Arc<ElementWaitService>,
    expectations: Arc<ExpectationService>,
}

impl WaitForTool {
    pub fn new(waits: Arc<ElementWaitService>, expectations: Arc<ExpectationService>) -> Self {
        Self {
            waits,
            expectations,
        }
    }
}

#[async_trait]
impl TypedTool for WaitForTool {
    type Arguments = WaitForArguments;

    const NAME: &'static str = "desk_wait_for";
    const DESCRIPTION: &'static str = "Wait until an element matching role/title/value appears \
        (e.g. after launching an app, opening a dialog or loading a page), then return the \
        matches with refs. With `state: \"hidden\"`, wait until no match is visible instead \
        (a spinner or dialog going away). Returns as soon as the state is reached; use this \
        instead of fixed delays.";
    const IS_READ_ONLY: bool = true;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(&self, arguments: WaitForArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let query = arguments.query.to_query();
        let timeout = timeout_from_seconds(arguments.timeout_seconds);
        let (query, limit, timeout) =
            match query.and_then(|query| Ok((query, match_limit(arguments.limit)?, timeout?))) {
                Ok(validated) => validated,
                Err(invalid_arguments) => return invalid_arguments.into(),
            };
        let selector = AppSelector::from_optional_name(arguments.app.as_deref());
        if arguments.state == WaitState::Hidden {
            let locator = Locator {
                app: selector,
                query,
                position: MatchPosition::Nth(0),
                timeout,
            };
            let expectations = self.expectations.clone();
            return run_blocking(move || {
                expectations.expect(&locator, &IsHidden)?;
                Ok(ToolOutput::text(format!(
                    "No element matching {} is visible.",
                    locator.query
                )))
            })
            .await
            .into();
        }
        let waits = self.waits.clone();
        run_blocking(move || waits.wait_for(&selector, &query, limit, timeout))
            .await
            .map(|results| ToolOutput::text(render_search_results(&results)))
            .into()
    }
}
