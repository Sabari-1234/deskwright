use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::ports::Clipboard;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct WriteClipboardArguments {
    /// The text to put on the clipboard.
    text: String,
}

/// Puts text on the clipboard. Uses the clipboard port directly: writing is the whole use
/// case.
pub struct WriteClipboardTool {
    clipboard: Arc<dyn Clipboard>,
}

impl WriteClipboardTool {
    pub fn new(clipboard: Arc<dyn Clipboard>) -> Self {
        Self { clipboard }
    }
}

#[async_trait]
impl TypedTool for WriteClipboardTool {
    type Arguments = WriteClipboardArguments;

    const NAME: &'static str = "desk_write_clipboard";
    const DESCRIPTION: &'static str = "Replace the clipboard's contents with `text`. Then \
        desk_press_keys \"cmd+v\" pastes it, which is the fastest way to enter long text.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(
        &self,
        arguments: WriteClipboardArguments,
        _context: &ToolContext<'_>,
    ) -> ToolOutput {
        let clipboard = self.clipboard.clone();
        // The confirmation omits the text, which may be a password.
        run_blocking(move || clipboard.write_text(&arguments.text))
            .await
            .map(|()| ToolOutput::text("Put the text on the clipboard."))
            .into()
    }
}
