use std::sync::Arc;
use std::time::Duration;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::native_element::NativeElement;
use crate::polling::wait_until;
use crate::ports::{ElementActions, ElementTreeReader, InputSynthesizer};
use crate::services::interaction::strategy_chain::NamedStrategy;

/// One way of putting text into an element.
pub trait TextEntryStrategy: NamedStrategy + Send + Sync {
    fn enter_text(&self, element: &NativeElement, text: &str) -> DeskwrightResult<()>;
}

/// Roles whose value is the text a user edits. Other elements (static text acting as a search
/// box, custom views) may accept a value write without showing it, so they are typed into.
const EDITABLE_TEXT_ROLES: [&str; 4] = ["AXTextField", "AXTextArea", "AXComboBox", "AXSearchField"];

/// Replaces the element's value through the accessibility API, then reads it back. Some
/// apps (many Electron apps) report success without applying the value; the read-back
/// turns that into a failure so the next strategy can take over. Only used for editable
/// text roles, because other elements can accept a write that changes nothing visible.
pub struct AccessibilityValueEntry {
    actions: Arc<dyn ElementActions>,
    tree: Arc<dyn ElementTreeReader>,
}

impl AccessibilityValueEntry {
    pub fn new(actions: Arc<dyn ElementActions>, tree: Arc<dyn ElementTreeReader>) -> Self {
        Self { actions, tree }
    }
}

impl NamedStrategy for AccessibilityValueEntry {
    fn name(&self) -> &'static str {
        "accessibility value"
    }
}

impl TextEntryStrategy for AccessibilityValueEntry {
    fn enter_text(&self, element: &NativeElement, text: &str) -> DeskwrightResult<()> {
        let role = self.tree.attributes(element)?.role;
        if !EDITABLE_TEXT_ROLES.contains(&role.as_str()) {
            return Err(DeskwrightError::operation_failed(
                "set the value",
                format!("{role} is not an editable text field"),
            ));
        }
        // Focusing is best-effort: many fields accept a value without focus, and an
        // ineffective write is caught by the read-back check below.
        let _ = self.actions.focus(element);
        self.actions.set_value(element, text)?;
        let applied_value = self.tree.attributes(element)?.value;
        // Secure (password) fields hide their value; when an editable field's value can't be
        // read, trust the successful write.
        match applied_value {
            Some(applied) if applied.trim() != text.trim() => {
                Err(DeskwrightError::operation_failed(
                    "verify the entered text",
                    "the app accepted the value but did not apply it",
                ))
            }
            _ => Ok(()),
        }
    }
}

/// Focuses the element and types the text with synthetic key events. Inserts at the cursor
/// instead of replacing the existing content. Refuses to type unless the element really has
/// keyboard focus afterwards, because the keys would otherwise land wherever focus already is.
pub struct KeyboardTextEntry {
    actions: Arc<dyn ElementActions>,
    tree: Arc<dyn ElementTreeReader>,
    input: Arc<dyn InputSynthesizer>,
    focus_settle_timeout: Duration,
}

/// How often focus is re-checked while waiting for it to arrive.
const FOCUS_POLL_INTERVAL: Duration = Duration::from_millis(20);

impl KeyboardTextEntry {
    /// `focus_settle_timeout` is how long to wait for focus after each attempt, because apps
    /// apply focus changes asynchronously.
    pub fn new(
        actions: Arc<dyn ElementActions>,
        tree: Arc<dyn ElementTreeReader>,
        input: Arc<dyn InputSynthesizer>,
        focus_settle_timeout: Duration,
    ) -> Self {
        Self {
            actions,
            tree,
            input,
            focus_settle_timeout,
        }
    }

    /// Asks for focus, then presses the element (like clicking into it) if that didn't work.
    fn give_focus(&self, element: &NativeElement) -> DeskwrightResult<()> {
        let _ = self.actions.focus(element);
        if self.has_focus_within_timeout(element) {
            return Ok(());
        }
        let _ = self.actions.press(element);
        if self.has_focus_within_timeout(element) {
            return Ok(());
        }
        Err(DeskwrightError::operation_failed(
            "focus the element",
            "it did not take keyboard focus",
        ))
    }

    fn has_focus_within_timeout(&self, element: &NativeElement) -> bool {
        wait_until(self.focus_settle_timeout, FOCUS_POLL_INTERVAL, || {
            self.tree
                .attributes(element)
                .is_ok_and(|attributes| attributes.is_focused)
        })
    }
}

impl NamedStrategy for KeyboardTextEntry {
    fn name(&self) -> &'static str {
        "keyboard typing"
    }
}

impl TextEntryStrategy for KeyboardTextEntry {
    fn enter_text(&self, element: &NativeElement, text: &str) -> DeskwrightResult<()> {
        self.give_focus(element)?;
        self.input.type_text(text)
    }
}
