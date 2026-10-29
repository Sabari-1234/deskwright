use deskwright_core::{DeskwrightError, DeskwrightResult};

const DEFAULT_MATCH_LIMIT: usize = 25;
const MAXIMUM_MATCH_LIMIT: usize = 200;

/// How many matching elements a search tool returns; keeps responses short.
pub fn match_limit(requested: Option<usize>) -> DeskwrightResult<usize> {
    match requested {
        None => Ok(DEFAULT_MATCH_LIMIT),
        Some(limit) if (1..=MAXIMUM_MATCH_LIMIT).contains(&limit) => Ok(limit),
        Some(_) => Err(DeskwrightError::InvalidInput(format!(
            "limit must be between 1 and {MAXIMUM_MATCH_LIMIT}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_accepts_limits_in_range() {
        assert_eq!(match_limit(None), Ok(DEFAULT_MATCH_LIMIT));
        assert_eq!(match_limit(Some(1)), Ok(1));
        assert_eq!(
            match_limit(Some(MAXIMUM_MATCH_LIMIT)),
            Ok(MAXIMUM_MATCH_LIMIT)
        );
    }

    #[test]
    fn rejects_zero_and_limits_above_the_maximum() {
        assert!(match_limit(Some(0)).is_err());
        assert!(match_limit(Some(MAXIMUM_MATCH_LIMIT + 1)).is_err());
    }
}
