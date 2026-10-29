use std::time::Duration;

use deskwright_core::{DeskwrightError, DeskwrightResult};

const DEFAULT_TIMEOUT_SECONDS: f64 = 5.0;

/// Converts a `timeoutSeconds` argument, defaulting to 5 seconds.
pub fn timeout_from_seconds(seconds: Option<f64>) -> DeskwrightResult<Duration> {
    let seconds = seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS);
    Duration::try_from_secs_f64(seconds).map_err(|_| {
        DeskwrightError::InvalidInput(format!(
            "timeoutSeconds must be a non-negative number, got {seconds}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_five_seconds_and_accepts_fractions() {
        assert_eq!(timeout_from_seconds(None), Ok(Duration::from_secs(5)));
        assert_eq!(
            timeout_from_seconds(Some(0.5)),
            Ok(Duration::from_millis(500))
        );
    }

    #[test]
    fn rejects_negative_and_non_finite_timeouts() {
        assert!(timeout_from_seconds(Some(-1.0)).is_err());
        assert!(timeout_from_seconds(Some(f64::NAN)).is_err());
    }
}
