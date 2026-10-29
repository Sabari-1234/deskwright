use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use deskwright_core::model::geometry::{Point, Size};
use deskwright_core::model::native_element::NativeElement;
use deskwright_core::ports::WindowController;
use deskwright_core::{DeskwrightError, DeskwrightResult};

use super::ax_element::AxElement;
use super::ax_names;
use super::ax_value_conversion::{from_point, from_size, to_bool};

/// [`WindowController`] over the macOS Accessibility API.
#[derive(Debug, Default)]
pub struct AxWindowController;

impl AxWindowController {
    pub fn new() -> Self {
        Self
    }
}

impl WindowController for AxWindowController {
    fn is_minimized(&self, window: &NativeElement) -> DeskwrightResult<bool> {
        let value = AxElement::from_native(window)?
            .attribute(ax_names::MINIMIZED)
            .map_err(|code| code.into_deskwright_error("read whether the window is minimized"))?;
        Ok(value.as_ref().and_then(to_bool).unwrap_or(false))
    }

    fn set_minimized(&self, window: &NativeElement, should_minimize: bool) -> DeskwrightResult<()> {
        let flag = if should_minimize {
            CFBoolean::true_value()
        } else {
            CFBoolean::false_value()
        };
        AxElement::from_native(window)?
            .set_attribute(ax_names::MINIMIZED, &flag.as_CFType())
            .map_err(|code| code.into_deskwright_error("minimize or restore the window"))
    }

    fn move_to(&self, window: &NativeElement, origin: Point) -> DeskwrightResult<()> {
        AxElement::from_native(window)?
            .set_attribute(ax_names::POSITION, &from_point(origin))
            .map_err(|code| code.into_deskwright_error("move the window"))
    }

    fn resize(&self, window: &NativeElement, size: Size) -> DeskwrightResult<()> {
        AxElement::from_native(window)?
            .set_attribute(ax_names::SIZE, &from_size(size))
            .map_err(|code| code.into_deskwright_error("resize the window"))
    }

    fn close(&self, window: &NativeElement) -> DeskwrightResult<()> {
        let close_button = AxElement::from_native(window)?
            .element(ax_names::CLOSE_BUTTON)
            .map_err(|code| code.into_deskwright_error("find the window's close button"))?
            .ok_or_else(|| {
                DeskwrightError::operation_failed("close the window", "it has no close button")
            })?;
        close_button
            .perform_action(ax_names::PRESS_ACTION)
            .map_err(|code| code.into_deskwright_error("close the window"))
    }

    fn raise(&self, window: &NativeElement) -> DeskwrightResult<()> {
        AxElement::from_native(window)?
            .perform_action(ax_names::RAISE_ACTION)
            .map_err(|code| code.into_deskwright_error("raise the window"))
    }
}
