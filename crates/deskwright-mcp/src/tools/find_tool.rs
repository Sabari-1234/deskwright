use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::model::app::AppSelector;
use deskwright_core::services::element_search_service::ElementSearchService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_outline::render_search_results;
use crate::match_limit::match_limit;
use crate::output_format::OutputFormat;
use crate::query_arguments::QueryArguments;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct FindArguments {
    #[serde(flatten)]
    query: QueryArguments,
    /// App name or bundle ID. Omit for the frontmost app.
    app: Option<String>,
    /// Maximum matches to return (1-200, default 25).
    limit: Option<usize>,
    /// "text" (default): one line per match. "json": full data with bounds.
    #[serde(default)]
    format: OutputFormat,
}

/// Finds elements matching role/title/value criteria.
pub struct FindTool {
    search: Arc<ElementSearchService>,
}

impl FindTool {
    pub fn new(search: Arc<ElementSearchService>) -> Self {
        Self { search }
    }
}

#[async_trait]
impl TypedTool for FindTool {
    type Arguments = FindArguments;

    const NAME: &'static str = "desk_find";
    const DESCRIPTION: &'static str = "Find elements in an app by role, title and/or value \
        (all given criteria must match). Returns matching elements with refs. Takes a fresh \
        snapshot, so refs from earlier snapshots stop working.";
    const IS_READ_ONLY: bool = true;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(&self, arguments: FindArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let query = arguments.query.to_query();
        let (query, limit) =
            match query.and_then(|query| Ok((query, match_limit(arguments.limit)?))) {
                Ok(validated) => validated,
                Err(invalid_arguments) => return invalid_arguments.into(),
            };
        let selector = AppSelector::from_optional_name(arguments.app.as_deref());
        let search = self.search.clone();
        let format = arguments.format;
        run_blocking(move || search.find(&selector, &query, limit))
            .await
            .map(|results| match format {
                OutputFormat::Text => ToolOutput::text(render_search_results(&results)),
                OutputFormat::Json => ToolOutput::json(&results.matches),
            })
            .into()
    }
}
