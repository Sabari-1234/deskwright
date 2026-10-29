use crate::error::DeskwrightResult;
use crate::model::app::ProcessId;
use crate::model::encoded_image::{CaptureOptions, EncodedImage};
use crate::model::geometry::Bounds;

/// Captures pixels from the screen.
pub trait ScreenCapturer: Send + Sync {
    /// Captures the main display.
    fn capture_main_display(&self, options: &CaptureOptions) -> DeskwrightResult<EncodedImage>;

    /// Captures a region given in global screen coordinates.
    fn capture_region(
        &self,
        region: Bounds,
        options: &CaptureOptions,
    ) -> DeskwrightResult<EncodedImage>;

    /// Captures the window of `process_id` at `bounds` from its own pixels, so other windows
    /// covering it don't appear. Fails when the window isn't on screen (another Space, hidden
    /// by Stage Manager) rather than returning whatever else is shown at `bounds`.
    fn capture_window(
        &self,
        process_id: ProcessId,
        bounds: Bounds,
        options: &CaptureOptions,
    ) -> DeskwrightResult<EncodedImage>;
}
