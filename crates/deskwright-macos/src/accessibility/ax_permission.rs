use deskwright_core::ports::AccessibilityPermission;

use super::ffi;

/// [`AccessibilityPermission`] backed by `AXIsProcessTrusted`. Never shows the system prompt,
/// which would block a server started in the background by an MCP client.
#[derive(Debug, Default)]
pub struct AxAccessibilityPermission;

impl AxAccessibilityPermission {
    pub fn new() -> Self {
        Self
    }
}

impl AccessibilityPermission for AxAccessibilityPermission {
    fn is_granted(&self) -> bool {
        // SAFETY: AXIsProcessTrusted has no preconditions.
        unsafe { ffi::AXIsProcessTrusted() != 0 }
    }
}
