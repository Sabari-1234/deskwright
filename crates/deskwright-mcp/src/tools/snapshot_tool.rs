use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::model::app::AppSelector;
use deskwright_core::model::tree_depth::TreeDepth;
use deskwright_core::services::snapshot_service::SnapshotService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_outline::render_snapshot;
use crate::output_format::OutputFormat;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotArguments {
    /// App name or bundle ID, e.g. "Safari" or "com.apple.Safari". Omit for the frontmost app.
    app: Option<String>,
    /// How many levels below each window to read (1-50, default 10). Lower is faster.
    max_depth: Option<usize>,
    /// "text" (default): compact outline, one line per element. "json": full data with bounds.
    #[serde(default)]
    format: OutputFormat,
}

/// Returns an app's UI element tree with refs for the other tools.
pub struct SnapshotTool {
    snapshots: Arc<SnapshotService>,
}

impl SnapshotTool {
    pub fn new(snapshots: Arc<SnapshotService>) -> Self {
        Self { snapshots }
    }
}

#[async_trait]
impl TypedTool for SnapshotTool {
    type Arguments = SnapshotArguments;

    const NAME: &'static str = "desk_snapshot";
    const DESCRIPTION: &'static str = "Read an app's UI element tree as a compact outline \
        (roles, titles, values; use format \"json\" for bounds). Every element gets a ref like 'e12' for desk_click, desk_type, desk_read \
        and desk_screenshot. Refs from earlier snapshots stop working, so take a new snapshot \
        whenever the UI changes. To locate specific elements, desk_find is faster and shorter.";
    const IS_READ_ONLY: bool = true;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(&self, arguments: SnapshotArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let depth = match arguments.max_depth.map(TreeDepth::new).transpose() {
            Ok(depth) => depth.unwrap_or_default(),
            Err(invalid_depth) => return invalid_depth.into(),
        };
        let selector = AppSelector::from_optional_name(arguments.app.as_deref());
        let snapshots = self.snapshots.clone();
        run_blocking(move || snapshots.capture(&selector, depth))
            .await
            .map(|snapshot| match arguments.format {
                OutputFormat::Text => ToolOutput::text(render_snapshot(&snapshot)),
                OutputFormat::Json => ToolOutput::json(&snapshot),
            })
            .into()
    }
}
