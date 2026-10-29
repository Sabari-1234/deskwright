use deskwright_core::{DeskwrightError, Permission};

use super::ffi::RawAxError;

/// A non-success `AXError` returned by the Accessibility API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxErrorCode(RawAxError);

impl AxErrorCode {
    const SUCCESS: RawAxError = 0;
    const GENERAL_FAILURE: RawAxError = -25200;
    const ILLEGAL_ARGUMENT: RawAxError = -25201;
    const INVALID_UI_ELEMENT: RawAxError = -25202;
    const CANNOT_COMPLETE: RawAxError = -25204;
    const ATTRIBUTE_UNSUPPORTED: RawAxError = -25205;
    const ACTION_UNSUPPORTED: RawAxError = -25206;
    const NOT_IMPLEMENTED: RawAxError = -25208;
    const API_DISABLED: RawAxError = -25211;
    const NO_VALUE: RawAxError = -25212;

    pub fn check(raw_error: RawAxError) -> Result<(), Self> {
        if raw_error == Self::SUCCESS {
            Ok(())
        } else {
            Err(Self(raw_error))
        }
    }

    /// True when the element simply has no such attribute, which callers treat as "absent".
    pub fn means_value_absent(self) -> bool {
        matches!(
            self.0,
            Self::NO_VALUE | Self::ATTRIBUTE_UNSUPPORTED | Self::NOT_IMPLEMENTED
        )
    }

    /// True when a position query found nothing: points outside every display (such as the
    /// gap between two monitors) report a general failure instead of an empty result.
    pub fn means_nothing_at_position(self) -> bool {
        self.0 == Self::GENERAL_FAILURE || self.means_value_absent()
    }

    pub fn into_deskwright_error(self, operation: &'static str) -> DeskwrightError {
        let reason = match self.0 {
            Self::API_DISABLED => {
                return DeskwrightError::PermissionDenied {
                    permission: Permission::Accessibility,
                };
            }
            Self::INVALID_UI_ELEMENT => {
                "the element no longer exists; take a new snapshot".to_owned()
            }
            Self::CANNOT_COMPLETE => "the app did not respond in time".to_owned(),
            Self::ACTION_UNSUPPORTED => "the element does not support this action".to_owned(),
            Self::ATTRIBUTE_UNSUPPORTED => "the element does not support this attribute".to_owned(),
            Self::ILLEGAL_ARGUMENT => "the app rejected the request".to_owned(),
            other => format!("Accessibility error {other}"),
        };
        DeskwrightError::operation_failed(operation, reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason_for(raw_error: RawAxError) -> String {
        AxErrorCode(raw_error)
            .into_deskwright_error("press the element")
            .to_string()
    }

    #[test]
    fn success_is_not_an_error() {
        assert_eq!(AxErrorCode::check(AxErrorCode::SUCCESS), Ok(()));
        assert_eq!(
            AxErrorCode::check(AxErrorCode::CANNOT_COMPLETE),
            Err(AxErrorCode(AxErrorCode::CANNOT_COMPLETE))
        );
    }

    #[test]
    fn treats_missing_attributes_as_absent_values() {
        for absent in [
            AxErrorCode::NO_VALUE,
            AxErrorCode::ATTRIBUTE_UNSUPPORTED,
            AxErrorCode::NOT_IMPLEMENTED,
        ] {
            assert!(AxErrorCode(absent).means_value_absent());
        }
        assert!(!AxErrorCode(AxErrorCode::INVALID_UI_ELEMENT).means_value_absent());
    }

    #[test]
    fn a_general_failure_at_a_position_means_nothing_is_there() {
        assert!(AxErrorCode(AxErrorCode::GENERAL_FAILURE).means_nothing_at_position());
        assert!(AxErrorCode(AxErrorCode::NO_VALUE).means_nothing_at_position());
        assert!(!AxErrorCode(AxErrorCode::API_DISABLED).means_nothing_at_position());
    }

    #[test]
    fn disabled_api_means_accessibility_permission_is_missing() {
        assert_eq!(
            AxErrorCode(AxErrorCode::API_DISABLED).into_deskwright_error("read"),
            DeskwrightError::PermissionDenied {
                permission: Permission::Accessibility
            }
        );
    }

    #[test]
    fn explains_common_failures_in_plain_words() {
        assert!(reason_for(AxErrorCode::INVALID_UI_ELEMENT).contains("no longer exists"));
        assert!(reason_for(AxErrorCode::CANNOT_COMPLETE).contains("did not respond"));
        assert!(
            reason_for(AxErrorCode::ACTION_UNSUPPORTED).contains("does not support this action")
        );
        assert!(
            reason_for(AxErrorCode::ATTRIBUTE_UNSUPPORTED)
                .contains("does not support this attribute")
        );
        assert!(reason_for(AxErrorCode::ILLEGAL_ARGUMENT).contains("rejected the request"));
    }

    #[test]
    fn reports_unknown_codes_by_number() {
        assert_eq!(
            reason_for(-25299),
            "press the element failed: Accessibility error -25299"
        );
    }
}
