use crate::error::DeskwrightResult;
use crate::model::native_element::NativeElement;

/// Acts on an element through the platform's accessibility API (no synthetic input).
pub trait ElementActions: Send + Sync {
    /// Performs the element's default action, like clicking a button or choosing a menu item.
    fn press(&self, element: &NativeElement) -> DeskwrightResult<()>;

    fn focus(&self, element: &NativeElement) -> DeskwrightResult<()>;

    /// Replaces the element's value, such as the text of a text field.
    fn set_value(&self, element: &NativeElement, value: &str) -> DeskwrightResult<()>;

    /// Opens the element's context menu, the accessibility equivalent of a right-click.
    fn show_menu(&self, element: &NativeElement) -> DeskwrightResult<()>;

    /// Dismisses an open menu without choosing anything, like pressing Escape in it.
    fn cancel(&self, element: &NativeElement) -> DeskwrightResult<()>;

    /// Scrolls the element's containers until it is visible.
    fn scroll_into_view(&self, element: &NativeElement) -> DeskwrightResult<()>;
}
