use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::model::app::AppSelector;
use deskwright_core::model::element::ElementRef;
use deskwright_core::services::locator_suggestion_service::LocatorSuggestionService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct LocatorArguments {
    /// Element ref from the latest snapshot or find, e.g. "e12".
    #[serde(rename = "ref")]
    element_ref: String,
    /// The app the snapshot was taken of. Omit for the frontmost app.
    app: Option<String>,
}

/// Suggests a stable locator for an element, like Playwright's "pick locator".
pub struct LocatorTool {
    suggestions: Arc<LocatorSuggestionService>,
}

impl LocatorTool {
    pub fn new(suggestions: Arc<LocatorSuggestionService>) -> Self {
        Self { suggestions }
    }
}

#[async_trait]
impl TypedTool for LocatorTool {
    type Arguments = LocatorArguments;

    const NAME: &'static str = "desk_locator";
    const DESCRIPTION: &'static str = "Turn a snapshot ref into the most stable locator that \
        finds that element (identifier, else role + exact title or placeholder, else role, \
        with nth when needed), as JSON to paste into flows and batches instead of refs, which \
        change. Takes new snapshots, so older refs are stale afterwards.";
    const IS_READ_ONLY: bool = true;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(&self, arguments: LocatorArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let element_ref = match ElementRef::parse(&arguments.element_ref) {
            Ok(element_ref) => element_ref,
            Err(invalid_ref) => return invalid_ref.into(),
        };
        let app = AppSelector::from_optional_name(arguments.app.as_deref());
        let suggestions = self.suggestions.clone();
        let app_name = arguments.app;
        run_blocking(move || suggestions.suggest(&element_ref, &app))
            .await
            .map(|locator| {
                let mut json = serde_json::to_value(&locator).unwrap_or_default();
                if let (Some(app_name), Some(fields)) = (app_name, json.as_object_mut()) {
                    fields.insert("app".to_owned(), app_name.into());
                }
                ToolOutput::text(json.to_string())
            })
            .into()
    }
}
