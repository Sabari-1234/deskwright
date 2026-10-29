use deskwright_core::model::app::ProcessId;
use deskwright_core::model::encoded_image::{CaptureOptions, EncodedImage};
use deskwright_core::model::geometry::Bounds;
use deskwright_core::ports::ScreenCapturer;
use deskwright_core::{DeskwrightError, DeskwrightResult};
use xcap::{Monitor, Window};

use crate::display_region::DisplayRegion;
use crate::image_encoding::encode;
use crate::window_match::{WindowCandidate, find_window};

/// [`ScreenCapturer`] backed by `xcap`. Kept to thin calls into the library; clipping,
/// window matching, scaling and encoding live in their own tested modules.
#[derive(Debug, Default)]
pub struct XcapScreenCapturer;

impl XcapScreenCapturer {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenCapturer for XcapScreenCapturer {
    fn capture_main_display(&self, options: &CaptureOptions) -> DeskwrightResult<EncodedImage> {
        let monitors = Monitor::all().map_err(capture_failed)?;
        let primary = monitors
            .into_iter()
            .find(|monitor| monitor.is_primary().unwrap_or(false))
            .ok_or_else(|| {
                DeskwrightError::operation_failed("capture screen", "no primary display found")
            })?;
        let point_size = (
            primary.width().map_err(capture_failed)?,
            primary.height().map_err(capture_failed)?,
        );
        encode(
            primary.capture_image().map_err(capture_failed)?,
            point_size,
            options,
        )
    }

    fn capture_region(
        &self,
        region: Bounds,
        options: &CaptureOptions,
    ) -> DeskwrightResult<EncodedImage> {
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
        encode(
            image,
            (display_region.width, display_region.height),
            options,
        )
    }

    /// Uses the window's own pixels, so covering windows (such as the AI app's) don't appear.
    /// The window server only lists windows that are on screen; for any other window the
    /// screen area at `bounds` shows a different app, so it fails instead.
    fn capture_window(
        &self,
        process_id: ProcessId,
        bounds: Bounds,
        options: &CaptureOptions,
    ) -> DeskwrightResult<EncodedImage> {
        let windows = Window::all().map_err(capture_failed)?;
        let candidates: Vec<WindowCandidate> = windows
            .iter()
            .map(|window| WindowCandidate {
                process_id: ProcessId(window.pid().map(|pid| pid as i32).unwrap_or(-1)),
                bounds: window_bounds(window).unwrap_or(Bounds {
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 0.0,
                }),
            })
            .collect();
        let index = find_window(&candidates, process_id, bounds).ok_or_else(|| {
            DeskwrightError::operation_failed(
                "capture the window",
                "it is not on screen (on another Space, or hidden by Stage Manager); bring it \
                 to the front with desk_window action \"raise\" first",
            )
        })?;
        let point_size = (bounds.width.round() as u32, bounds.height.round() as u32);
        encode(
            windows[index].capture_image().map_err(capture_failed)?,
            point_size,
            options,
        )
    }
}

fn window_bounds(window: &Window) -> Result<Bounds, xcap::XCapError> {
    Ok(Bounds {
        x: f64::from(window.x()?),
        y: f64::from(window.y()?),
        width: f64::from(window.width()?),
        height: f64::from(window.height()?),
    })
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
