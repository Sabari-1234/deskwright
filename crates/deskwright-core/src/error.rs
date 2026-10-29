use std::fmt;

use thiserror::Error;

use crate::model::element::ElementRef;

pub type DeskwrightResult<T> = Result<T, DeskwrightError>;

/// An operating-system permission the server needs for some operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Accessibility,
    ScreenRecording,
}

impl fmt::Display for Permission {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Accessibility => "Accessibility",
            Self::ScreenRecording => "Screen Recording",
        };
        formatter.write_str(label)
    }
}

/// Every failure the core and its adapters can report, phrased for the end user.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DeskwrightError {
    #[error(
        "{permission} permission is not granted. Grant it in System Settings > Privacy & Security > {permission}, then restart the server."
    )]
    PermissionDenied { permission: Permission },

    #[error("no running app matches '{0}'")]
    AppNotFound(String),

    #[error("no installed app matches '{0}'")]
    AppNotInstalled(String),

    #[error("there is no frontmost app")]
    NoFrontmostApp,

    #[error("'{0}' is not the frontmost app; bring it to the front with desk_launch_app first")]
    AppNotFrontmost(String),

    #[error("unknown element ref '{0}'; take a new snapshot first")]
    UnknownElementRef(ElementRef),

    #[error("element {0} has no on-screen bounds")]
    ElementHasNoBounds(ElementRef),

    #[error("menu item '{missing_item}' was not found while following '{path}'")]
    MenuItemNotFound { path: String, missing_item: String },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("{operation} failed: {reason}")]
    OperationFailed {
        operation: &'static str,
        reason: String,
    },

    #[error("could not {action}; every strategy failed: {}", attempts.join("; "))]
    AllStrategiesFailed {
        action: &'static str,
        attempts: Vec<String>,
    },

    #[error("no element matching {query} appeared within {timeout_milliseconds} ms{last_problem}")]
    WaitTimedOut {
        query: String,
        timeout_milliseconds: u128,
        /// Why the last attempt failed, when it failed rather than finding nothing.
        last_problem: String,
    },

    #[error("expected {subject} {expectation} within {timeout_milliseconds} ms, but {observed}")]
    ExpectationFailed {
        subject: String,
        expectation: String,
        timeout_milliseconds: u128,
        /// What was seen on the last check.
        observed: String,
    },

    #[error("script timed out after {timeout_seconds} seconds")]
    ScriptTimedOut { timeout_seconds: u64 },

    #[error("script failed: {0}")]
    ScriptFailed(String),
}

impl DeskwrightError {
    pub fn operation_failed(operation: &'static str, reason: impl Into<String>) -> Self {
        Self::OperationFailed {
            operation,
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_denied_tells_the_user_where_to_grant_it() {
        let message = DeskwrightError::PermissionDenied {
            permission: Permission::ScreenRecording,
        }
        .to_string();

        assert!(message.starts_with("Screen Recording permission is not granted"));
        assert!(message.contains("Privacy & Security > Screen Recording"));
    }

    #[test]
    fn accessibility_permission_has_a_readable_name() {
        assert_eq!(Permission::Accessibility.to_string(), "Accessibility");
    }

    #[test]
    fn operation_failed_names_the_operation_and_the_reason() {
        let error = DeskwrightError::operation_failed("press the element", "it is disabled");

        assert_eq!(
            error.to_string(),
            "press the element failed: it is disabled"
        );
    }

    #[test]
    fn all_strategies_failed_lists_every_attempt() {
        let error = DeskwrightError::AllStrategiesFailed {
            action: "click the element",
            attempts: vec!["press: no".to_owned(), "pointer: hidden".to_owned()],
        };

        assert_eq!(
            error.to_string(),
            "could not click the element; every strategy failed: press: no; pointer: hidden"
        );
    }
}
