use crate::error::DeskwrightResult;
use crate::model::web_address::WebAddress;

/// Starts apps and brings them to the front.
pub trait AppLauncher: Send + Sync {
    /// Launches the app named by its display name or bundle ID if it isn't running, brings
    /// it to the front, and optionally opens `address` in it. Returns once the app is
    /// frontmost, or fails if it doesn't become frontmost in time.
    fn open(&self, name_or_bundle_id: &str, address: Option<&WebAddress>) -> DeskwrightResult<()>;
}
