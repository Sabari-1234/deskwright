use std::sync::Arc;

use crate::error::DeskwrightResult;
use crate::model::native_element::NativeElement;
use crate::ports::{ElementActions, InputSynthesizer};
use crate::services::interaction::pointer_target::PointerTarget;
use crate::services::interaction::strategy_chain::NamedStrategy;

/// One way of clicking an element.
pub trait ClickStrategy: NamedStrategy + Send + Sync {
    fn click(&self, element: &NativeElement) -> DeskwrightResult<()>;
}

/// Performs the element's accessibility "press" action. Precise and works even when the
/// window is covered, but some custom controls don't implement it.
pub struct AccessibilityPressClick {
    actions: Arc<dyn ElementActions>,
}

impl AccessibilityPressClick {
    pub fn new(actions: Arc<dyn ElementActions>) -> Self {
        Self { actions }
    }
}

impl NamedStrategy for AccessibilityPressClick {
    fn name(&self) -> &'static str {
        "accessibility press"
    }
}

impl ClickStrategy for AccessibilityPressClick {
    fn click(&self, element: &NativeElement) -> DeskwrightResult<()> {
        self.actions.press(element)
    }
}

/// Opens the element's context menu through accessibility: a right-click without moving the
/// pointer.
pub struct AccessibilityMenuClick {
    actions: Arc<dyn ElementActions>,
}

impl AccessibilityMenuClick {
    pub fn new(actions: Arc<dyn ElementActions>) -> Self {
        Self { actions }
    }
}

impl NamedStrategy for AccessibilityMenuClick {
    fn name(&self) -> &'static str {
        "accessibility menu"
    }
}

impl ClickStrategy for AccessibilityMenuClick {
    fn click(&self, element: &NativeElement) -> DeskwrightResult<()> {
        self.actions.show_menu(element)
    }
}

/// Which mouse gesture a [`PointerClick`] performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerGesture {
    Single,
    Double,
    Secondary,
}

/// Sends a synthetic mouse gesture to the center of the element's on-screen bounds. Works
/// for any visible element, but refuses when another app's window covers that point.
pub struct PointerClick {
    target: PointerTarget,
    input: Arc<dyn InputSynthesizer>,
    gesture: PointerGesture,
}

impl PointerClick {
    pub fn new(
        target: PointerTarget,
        input: Arc<dyn InputSynthesizer>,
        gesture: PointerGesture,
    ) -> Self {
        Self {
            target,
            input,
            gesture,
        }
    }
}

impl NamedStrategy for PointerClick {
    fn name(&self) -> &'static str {
        match self.gesture {
            PointerGesture::Single => "pointer click",
            PointerGesture::Double => "pointer double-click",
            PointerGesture::Secondary => "pointer right-click",
        }
    }
}

impl ClickStrategy for PointerClick {
    fn click(&self, element: &NativeElement) -> DeskwrightResult<()> {
        let point = self.target.safe_point(element)?;
        match self.gesture {
            PointerGesture::Single => self.input.click_at(point),
            PointerGesture::Double => self.input.double_click_at(point),
            PointerGesture::Secondary => self.input.right_click_at(point),
        }
    }
}
