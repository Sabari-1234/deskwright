use crate::error::DeskwrightResult;
use crate::model::app::ProcessId;
use crate::model::native_element::NativeElement;

/// Finds an app's menu bar, the root for menu navigation.
pub trait MenuBarLocator: Send + Sync {
    fn menu_bar(&self, process_id: ProcessId) -> DeskwrightResult<NativeElement>;
}
