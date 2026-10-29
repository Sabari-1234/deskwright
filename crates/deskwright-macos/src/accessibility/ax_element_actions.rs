use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::string::CFString;
use deskwright_core::DeskwrightResult;
use deskwright_core::model::native_element::NativeElement;
use deskwright_core::ports::ElementActions;

use super::ax_element::AxElement;
use super::ax_names;

/// [`ElementActions`] over the macOS Accessibility API.
#[derive(Debug, Default)]
pub struct AxElementActions;

impl AxElementActions {
    pub fn new() -> Self {
        Self
    }
}

impl ElementActions for AxElementActions {
    fn press(&self, element: &NativeElement) -> DeskwrightResult<()> {
        AxElement::from_native(element)?
            .perform_action(ax_names::PRESS_ACTION)
            .map_err(|code| code.into_deskwright_error("press the element"))
    }

    fn focus(&self, element: &NativeElement) -> DeskwrightResult<()> {
        AxElement::from_native(element)?
            .set_attribute(ax_names::FOCUSED, &CFBoolean::true_value().as_CFType())
            .map_err(|code| code.into_deskwright_error("focus the element"))
    }

    fn set_value(&self, element: &NativeElement, value: &str) -> DeskwrightResult<()> {
        AxElement::from_native(element)?
            .set_attribute(ax_names::VALUE, &CFString::new(value).as_CFType())
            .map_err(|code| code.into_deskwright_error("set the element's value"))
    }

    fn show_menu(&self, element: &NativeElement) -> DeskwrightResult<()> {
        AxElement::from_native(element)?
            .perform_action(ax_names::SHOW_MENU_ACTION)
            .map_err(|code| code.into_deskwright_error("show the element's menu"))
    }

    fn cancel(&self, element: &NativeElement) -> DeskwrightResult<()> {
        AxElement::from_native(element)?
            .perform_action(ax_names::CANCEL_ACTION)
            .map_err(|code| code.into_deskwright_error("close the menu"))
    }

    fn scroll_into_view(&self, element: &NativeElement) -> DeskwrightResult<()> {
        AxElement::from_native(element)?
            .perform_action(ax_names::SCROLL_TO_VISIBLE_ACTION)
            .map_err(|code| code.into_deskwright_error("scroll the element into view"))
    }
}
