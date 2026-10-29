use crate::error::DeskwrightResult;
use crate::model::geometry::Point;
use crate::model::native_element::NativeElement;

/// Answers what is actually on screen at a point, so synthetic clicks never land on a window
/// that covers the intended element.
pub trait HitTester: Send + Sync {
    /// True when the topmost thing at `point` belongs to the same app as `element`.
    fn is_app_of_element_on_top_at(
        &self,
        element: &NativeElement,
        point: Point,
    ) -> DeskwrightResult<bool>;
}
