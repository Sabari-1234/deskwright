use crate::error::DeskwrightResult;

/// The system clipboard's plain-text contents.
pub trait Clipboard: Send + Sync {
    /// The clipboard's text, or `None` when it holds no text.
    fn read_text(&self) -> DeskwrightResult<Option<String>>;

    /// Replaces the clipboard's contents with `text`.
    fn write_text(&self, text: &str) -> DeskwrightResult<()>;
}
