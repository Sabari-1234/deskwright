use crate::error::DeskwrightResult;
use crate::model::geometry::Bounds;
use crate::model::png_image::PngImage;

/// Captures pixels from the screen.
pub trait ScreenCapturer: Send + Sync {
    /// Captures the main display.
    fn capture_main_display(&self) -> DeskwrightResult<PngImage>;

    /// Captures a region given in global screen coordinates.
    fn capture_region(&self, region: Bounds) -> DeskwrightResult<PngImage>;
}
