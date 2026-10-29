use std::sync::Arc;

use crate::element_registry::ElementRegistry;
use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::app::AppSelector;
use crate::model::element::ElementRef;
use crate::model::encoded_image::{CaptureOptions, EncodedImage};
use crate::model::window::WindowSelector;
use crate::ports::{ElementTreeReader, ScreenCapturer};
use crate::services::window_service::WindowService;

/// What a screenshot should show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenshotTarget {
    MainDisplay,
    Element(ElementRef),
    /// One of an app's windows, captured on its own even when other windows cover it.
    Window {
        app: AppSelector,
        window: WindowSelector,
    },
}

/// Captures the screen, one element's on-screen area, or one window.
pub struct ScreenshotService {
    capturer: Arc<dyn ScreenCapturer>,
    registry: Arc<ElementRegistry>,
    tree: Arc<dyn ElementTreeReader>,
    windows: Arc<WindowService>,
}

impl ScreenshotService {
    pub fn new(
        capturer: Arc<dyn ScreenCapturer>,
        registry: Arc<ElementRegistry>,
        tree: Arc<dyn ElementTreeReader>,
        windows: Arc<WindowService>,
    ) -> Self {
        Self {
            capturer,
            registry,
            tree,
            windows,
        }
    }

    pub fn capture(
        &self,
        target: &ScreenshotTarget,
        options: &CaptureOptions,
    ) -> DeskwrightResult<EncodedImage> {
        match target {
            ScreenshotTarget::MainDisplay => self.capturer.capture_main_display(options),
            ScreenshotTarget::Element(element_ref) => self.capture_element(element_ref, options),
            ScreenshotTarget::Window { app, window } => self.capture_window(app, window, options),
        }
    }

    fn capture_element(
        &self,
        element_ref: &ElementRef,
        options: &CaptureOptions,
    ) -> DeskwrightResult<EncodedImage> {
        let element = self.registry.resolve(element_ref)?;
        let bounds = self
            .tree
            .on_screen_bounds(&element)?
            .ok_or(DeskwrightError::ElementHasNoBounds(*element_ref))?;
        self.capturer.capture_region(bounds, options)
    }

    fn capture_window(
        &self,
        app: &AppSelector,
        selector: &WindowSelector,
        options: &CaptureOptions,
    ) -> DeskwrightResult<EncodedImage> {
        let (app, window) = self.windows.find(app, selector)?;
        if window.is_minimized {
            return Err(DeskwrightError::operation_failed(
                "capture the window",
                "it is minimized; restore it with desk_window first",
            ));
        }
        let bounds = window
            .bounds
            .filter(|bounds| !bounds.is_empty())
            .ok_or_else(|| {
                DeskwrightError::operation_failed("capture the window", "it has no on-screen size")
            })?;
        self.capturer
            .capture_window(app.process_id, bounds, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::app::ProcessId;
    use crate::model::encoded_image::{CaptureResolution, ImageFormat};
    use crate::model::geometry::Bounds;
    use crate::services::app_resolver::AppResolver;
    use crate::testing::{FakeDesktop, FakeElementId, element};

    fn screenshot_service(
        desktop: &Arc<FakeDesktop>,
        registry: Arc<ElementRegistry>,
    ) -> ScreenshotService {
        let windows = WindowService::new(
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            desktop.clone(),
            desktop.clone(),
            std::time::Duration::ZERO,
        );
        ScreenshotService::new(
            desktop.clone(),
            registry,
            desktop.clone(),
            Arc::new(windows),
        )
    }

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
        let service = screenshot_service(&desktop, registry);

        service
            .capture(
                &ScreenshotTarget::Element(image_ref),
                &CaptureOptions::default(),
            )
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
        let service = screenshot_service(&desktop, registry);

        let error = service
            .capture(
                &ScreenshotTarget::Element(hidden_ref),
                &CaptureOptions::default(),
            )
            .unwrap_err();

        assert_eq!(error, DeskwrightError::ElementHasNoBounds(hidden_ref));
    }

    fn notes_with_window(
        desktop: &FakeDesktop,
        bounds: Option<Bounds>,
    ) -> (ProcessId, FakeElementId) {
        let process_id = desktop.add_app("Notes", None);
        let mut window = element("AXWindow", Some("Groceries"));
        window.bounds = bounds;
        (process_id, desktop.add_window(process_id, window))
    }

    fn notes_window() -> ScreenshotTarget {
        ScreenshotTarget::Window {
            app: AppSelector::NameOrBundleId("Notes".to_owned()),
            window: WindowSelector::default(),
        }
    }

    #[test]
    fn captures_a_window_by_its_process_and_bounds_with_the_options() {
        let desktop = Arc::new(FakeDesktop::new());
        let bounds = Bounds {
            x: 100.0,
            y: 50.0,
            width: 400.0,
            height: 300.0,
        };
        let (process_id, _) = notes_with_window(&desktop, Some(bounds));
        let options = CaptureOptions {
            resolution: CaptureResolution::Pixels,
            format: ImageFormat::Jpeg { quality: 60 },
        };

        let image = screenshot_service(&desktop, Arc::new(ElementRegistry::default()))
            .capture(&notes_window(), &options)
            .unwrap();

        assert_eq!(desktop.captured_windows(), [(process_id, bounds)]);
        assert_eq!(image.format(), ImageFormat::Jpeg { quality: 60 });
        assert_eq!(desktop.capture_options(), [options]);
    }

    #[test]
    fn refuses_minimized_and_unsized_windows() {
        let desktop = Arc::new(FakeDesktop::new());
        let (_, window) = notes_with_window(&desktop, None);
        let service = screenshot_service(&desktop, Arc::new(ElementRegistry::default()));

        let without_size = service
            .capture(&notes_window(), &CaptureOptions::default())
            .unwrap_err();
        desktop.set_minimized(window, true);
        let minimized = service
            .capture(&notes_window(), &CaptureOptions::default())
            .unwrap_err();

        assert!(without_size.to_string().contains("no on-screen size"));
        assert!(minimized.to_string().contains("it is minimized"));
        assert!(desktop.captured_windows().is_empty());
    }
}
