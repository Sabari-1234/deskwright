use deskwright_core::DeskwrightResult;
use deskwright_core::model::geometry::Point;
use deskwright_core::model::native_element::NativeElement;
use deskwright_core::ports::HitTester;

use super::ax_element::AxElement;
use crate::process_list::{executable_path, is_hosted_ui_service};

/// [`HitTester`] that asks the Accessibility API which element is topmost at a point and
/// compares the app that owns it with the app that owns the target element. UI drawn by a
/// system XPC service (such as a sandboxed app's Save panel) counts as the target app's own.
#[derive(Debug, Default)]
pub struct AxHitTester;

impl AxHitTester {
    pub fn new() -> Self {
        Self
    }
}

impl HitTester for AxHitTester {
    fn is_app_of_element_on_top_at(
        &self,
        element: &NativeElement,
        point: Point,
    ) -> DeskwrightResult<bool> {
        let target_process = AxElement::from_native(element)?
            .process_id()
            .map_err(|code| code.into_deskwright_error("identify the element's app"))?;
        let topmost = AxElement::system_wide()?
            .element_at_position(point)
            .map_err(|code| code.into_deskwright_error("find what is on screen at the element"))?;
        let Some(topmost_process) = topmost.and_then(|topmost| topmost.process_id().ok()) else {
            return Ok(false);
        };
        Ok(topmost_process == target_process
            || executable_path(topmost_process).is_some_and(|path| is_hosted_ui_service(&path)))
    }
}
