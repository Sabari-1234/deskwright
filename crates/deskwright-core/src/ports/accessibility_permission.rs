use crate::error::{DeskwrightError, DeskwrightResult, Permission};

/// Reports whether this process may read and control other apps' UI.
pub trait AccessibilityPermission: Send + Sync {
    fn is_granted(&self) -> bool;

    fn ensure_granted(&self) -> DeskwrightResult<()> {
        if self.is_granted() {
            Ok(())
        } else {
            Err(DeskwrightError::PermissionDenied {
                permission: Permission::Accessibility,
            })
        }
    }
}
