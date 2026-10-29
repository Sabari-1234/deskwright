use std::sync::Arc;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::geometry::Point;
use crate::model::native_element::NativeElement;
use crate::ports::{ElementTreeReader, HitTester};

/// Works out where synthetic pointer input for an element should go: the center of its
/// on-screen bounds, but only when the element's own app is on top there, so input can never
/// land in a covering window of another app.
#[derive(Clone)]
pub struct PointerTarget {
    tree: Arc<dyn ElementTreeReader>,
    hit_tester: Arc<dyn HitTester>,
}

impl PointerTarget {
    pub fn new(tree: Arc<dyn ElementTreeReader>, hit_tester: Arc<dyn HitTester>) -> Self {
        Self { tree, hit_tester }
    }

    /// The element's on-screen center, refusing when another app covers it.
    pub fn safe_point(&self, element: &NativeElement) -> DeskwrightResult<Point> {
        let point = self.visible_center(element)?;
        if self
            .hit_tester
            .is_app_of_element_on_top_at(element, point)?
        {
            Ok(point)
        } else {
            Err(DeskwrightError::operation_failed(
                "reach the element",
                "another app's window covers it; bring its window to the front first",
            ))
        }
    }

    /// The element's on-screen center without the covering check, for drop targets that may
    /// legitimately belong to another app.
    pub fn visible_center(&self, element: &NativeElement) -> DeskwrightResult<Point> {
        self.tree
            .on_screen_bounds(element)?
            .map(|bounds| bounds.center())
            .ok_or_else(|| {
                DeskwrightError::operation_failed(
                    "locate the element",
                    "it is not visible on screen",
                )
            })
    }
}
