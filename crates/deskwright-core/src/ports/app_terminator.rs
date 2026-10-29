use crate::error::DeskwrightResult;
use crate::model::app::ProcessId;

/// Asks running apps to quit.
pub trait AppTerminator: Send + Sync {
    /// Asks the app to quit, as its Quit menu item would (it may ask to save changes), or with
    /// `force`, ends it at once and loses unsaved work. Returns once the request is sent.
    fn quit(&self, process_id: ProcessId, force: bool) -> DeskwrightResult<()>;
}
