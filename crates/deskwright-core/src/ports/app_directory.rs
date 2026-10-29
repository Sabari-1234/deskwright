use crate::error::DeskwrightResult;
use crate::model::app::AppInfo;

/// Lists the user-facing apps that are currently running.
pub trait AppDirectory: Send + Sync {
    fn running_apps(&self) -> DeskwrightResult<Vec<AppInfo>>;

    fn frontmost_app(&self) -> DeskwrightResult<Option<AppInfo>>;
}
