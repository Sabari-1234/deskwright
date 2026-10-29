use crate::error::DeskwrightResult;
use crate::model::geometry::{Point, Size};
use crate::model::native_element::NativeElement;

/// Manages top-level windows through the platform's accessibility API.
pub trait WindowController: Send + Sync {
    fn is_minimized(&self, window: &NativeElement) -> DeskwrightResult<bool>;

    fn set_minimized(&self, window: &NativeElement, should_minimize: bool) -> DeskwrightResult<()>;

    /// Moves the window's top-left corner to `origin`, in global screen coordinates.
    fn move_to(&self, window: &NativeElement, origin: Point) -> DeskwrightResult<()>;

    fn resize(&self, window: &NativeElement, size: Size) -> DeskwrightResult<()>;

    /// Closes the window as its close button would, so the app may still ask to save.
    fn close(&self, window: &NativeElement) -> DeskwrightResult<()>;

    /// Makes the window the frontmost window of its app.
    fn raise(&self, window: &NativeElement) -> DeskwrightResult<()>;
}
