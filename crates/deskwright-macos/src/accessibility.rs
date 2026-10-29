//! Adapters over the macOS Accessibility API (`AXUIElement`).

mod ax_element;
mod ax_element_actions;
mod ax_error_code;
mod ax_hit_tester;
mod ax_menu_bar_locator;
mod ax_names;
mod ax_permission;
mod ax_tree_reader;
mod ax_value_conversion;
mod ax_window_controller;
mod ffi;

use std::time::Duration;

use deskwright_core::DeskwrightResult;

pub use ax_element_actions::AxElementActions;
pub use ax_hit_tester::AxHitTester;
pub use ax_menu_bar_locator::AxMenuBarLocator;
pub use ax_permission::AxAccessibilityPermission;
pub use ax_tree_reader::AxTreeReader;
pub use ax_window_controller::AxWindowController;

use ax_element::AxElement;

/// Sets how long Accessibility requests from this process wait for an unresponsive app
/// before failing. The system default (about six seconds) makes a hung app stall every tool.
pub fn configure_accessibility_timeout(timeout: Duration) -> DeskwrightResult<()> {
    AxElement::system_wide()?
        .set_messaging_timeout(timeout)
        .map_err(|code| code.into_deskwright_error("set the Accessibility timeout"))
}
