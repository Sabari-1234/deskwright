use deskwright_core::model::app::ProcessId;
use deskwright_core::model::native_element::NativeElement;
use deskwright_core::ports::MenuBarLocator;
use deskwright_core::{DeskwrightError, DeskwrightResult};

use super::ax_element::AxElement;
use super::ax_names;

/// [`MenuBarLocator`] over the macOS Accessibility API.
#[derive(Debug, Default)]
pub struct AxMenuBarLocator;

impl AxMenuBarLocator {
    pub fn new() -> Self {
        Self
    }
}

impl MenuBarLocator for AxMenuBarLocator {
    fn menu_bar(&self, process_id: ProcessId) -> DeskwrightResult<NativeElement> {
        AxElement::application(process_id)?
            .element(ax_names::MENU_BAR)
            .map_err(|code| code.into_deskwright_error("read the menu bar"))?
            .map(AxElement::into_native)
            .ok_or_else(|| {
                DeskwrightError::operation_failed("read the menu bar", "the app has no menu bar")
            })
    }
}
