use deskwright_core::model::geometry::Bounds;
use deskwright_core::model::png_image::PngImage;
use deskwright_core::ports::ScreenCapturer;
use deskwright_core::{DeskwrightError, DeskwrightResult};
use xcap::Monitor;

use crate::display_region::DisplayRegion;
use crate::png_encoding::encode_png;

/// [`ScreenCapturer`] backed by `xcap`. Kept to thin calls into the library; the clipping
/// math and PNG encoding live in their own tested modules.
#[derive(Debug, Default)]
pub struct XcapScreenCapturer;

impl XcapScreenCapturer {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenCapturer for XcapScreenCapturer {
    fn capture_main_display(&self) -> DeskwrightResult<PngImage> {
        let monitors = Monitor::all().map_err(capture_failed)?;
        let primary = monitors
            .into_iter()
            .find(|monitor| monitor.is_primary().unwrap_or(false))
            .ok_or_else(|| {
                DeskwrightError::operation_failed("capture screen", "no primary display found")
            })?;
        encode_png(&primary.capture_image().map_err(capture_failed)?)
    }

    fn capture_region(&self, region: Bounds) -> DeskwrightResult<PngImage> {
        let center = region.center();
        let monitor = Monitor::from_point(center.x.round() as i32, center.y.round() as i32)
            .map_err(capture_failed)?;
        let display_region = DisplayRegion::clip(region, display_bounds(&monitor)?)?;
        let image = monitor
            .capture_region(
                display_region.x,
                display_region.y,
                display_region.width,
                display_region.height,
            )
            .map_err(capture_failed)?;
        encode_png(&image)
    }
}

fn display_bounds(monitor: &Monitor) -> DeskwrightResult<Bounds> {
    Ok(Bounds {
        x: f64::from(monitor.x().map_err(capture_failed)?),
        y: f64::from(monitor.y().map_err(capture_failed)?),
        width: f64::from(monitor.width().map_err(capture_failed)?),
        height: f64::from(monitor.height().map_err(capture_failed)?),
    })
}

fn capture_failed(error: xcap::XCapError) -> DeskwrightError {
    DeskwrightError::operation_failed(
        "capture screen",
        format!("{error} (Screen Recording permission may be missing)"),
    )
}
