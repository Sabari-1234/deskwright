use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::ports::Clipboard;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct ReadClipboardArguments {}

/// Returns the clipboard's text. Uses the clipboard port directly: reading is the whole use
/// case.
pub struct ReadClipboardTool {
    clipboard: Arc<dyn Clipboard>,
}

impl ReadClipboardTool {
    pub fn new(clipboard: Arc<dyn Clipboard>) -> Self {
        Self { clipboard }
    }
}

#[async_trait]
impl TypedTool for ReadClipboardTool {
    type Arguments = ReadClipboardArguments;

    const NAME: &'static str = "desk_read_clipboard";
    const DESCRIPTION: &'static str = "Return the text on the clipboard, e.g. after pressing \
        Cmd+C in an app to copy a selection.";
    const IS_READ_ONLY: bool = true;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(
        &self,
        _arguments: ReadClipboardArguments,
        _context: &ToolContext<'_>,
    ) -> ToolOutput {
        let clipboard = self.clipboard.clone();
        run_blocking(move || clipboard.read_text())
            .await
            .map(|text| {
                ToolOutput::text(text.unwrap_or_else(|| "The clipboard holds no text.".to_owned()))
            })
            .into()
    }
}
