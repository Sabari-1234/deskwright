use std::sync::Arc;

use crate::element_registry::ElementRegistry;
use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::element::ElementRef;
use crate::model::png_image::PngImage;
use crate::ports::{ElementTreeReader, ScreenCapturer};

/// What a screenshot should show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenshotTarget {
    MainDisplay,
    Element(ElementRef),
}

/// Captures the screen or one element's on-screen area as a PNG.
pub struct ScreenshotService {
    capturer: Arc<dyn ScreenCapturer>,
    registry: Arc<ElementRegistry>,
    tree: Arc<dyn ElementTreeReader>,
}

impl ScreenshotService {
    pub fn new(
        capturer: Arc<dyn ScreenCapturer>,
        registry: Arc<ElementRegistry>,
        tree: Arc<dyn ElementTreeReader>,
    ) -> Self {
        Self {
            capturer,
            registry,
            tree,
        }
    }

    pub fn capture(&self, target: &ScreenshotTarget) -> DeskwrightResult<PngImage> {
        match target {
            ScreenshotTarget::MainDisplay => self.capturer.capture_main_display(),
            ScreenshotTarget::Element(element_ref) => self.capture_element(element_ref),
        }
    }

    fn capture_element(&self, element_ref: &ElementRef) -> DeskwrightResult<PngImage> {
        let element = self.registry.resolve(element_ref)?;
        let bounds = self
            .tree
            .on_screen_bounds(&element)?
            .ok_or(DeskwrightError::ElementHasNoBounds(*element_ref))?;
        self.capturer.capture_region(bounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::geometry::Bounds;
    use crate::testing::{FakeDesktop, element};

    #[test]
    fn captures_the_region_covered_by_the_element() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Preview", None);
        let element_bounds = Bounds {
            x: 5.0,
            y: 6.0,
            width: 70.0,
            height: 80.0,
        };
        let mut attributes = element("AXImage", None);
        attributes.bounds = Some(element_bounds);
        let image = desktop.add_window(process_id, attributes);
        let registry = Arc::new(ElementRegistry::default());
        let image_ref = desktop.register(&registry, image);
        let service = ScreenshotService::new(desktop.clone(), registry, desktop.clone());

        service
            .capture(&ScreenshotTarget::Element(image_ref))
            .unwrap();

        assert_eq!(desktop.captured_regions(), [element_bounds]);
    }

    #[test]
    fn rejects_elements_without_on_screen_bounds() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Preview", None);
        let hidden = desktop.add_window(process_id, element("AXGroup", None));
        let registry = Arc::new(ElementRegistry::default());
        let hidden_ref = desktop.register(&registry, hidden);
        let service = ScreenshotService::new(desktop.clone(), registry, desktop.clone());

        let error = service
            .capture(&ScreenshotTarget::Element(hidden_ref))
            .unwrap_err();

        assert_eq!(error, DeskwrightError::ElementHasNoBounds(hidden_ref));
    }
}
