use std::fmt;

use crate::error::{DeskwrightError, DeskwrightResult};

const ALLOWED_SCHEMES: [&str; 2] = ["https://", "http://"];

/// An http or https address to open in an app. Other schemes (`file:`, custom app schemes)
/// are rejected, because opening them can run local actions rather than show a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebAddress(String);

impl WebAddress {
    pub fn parse(raw: &str) -> DeskwrightResult<Self> {
        let trimmed = raw.trim();
        let lowercase = trimmed.to_lowercase();
        let host_part = ALLOWED_SCHEMES
            .iter()
            .find_map(|scheme| lowercase.strip_prefix(scheme));
        let is_valid = host_part
            .is_some_and(|rest| !rest.is_empty() && !trimmed.chars().any(char::is_whitespace));
        if !is_valid {
            return Err(DeskwrightError::InvalidInput(format!(
                "'{raw}' is not an http or https address without spaces"
            )));
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WebAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_http_and_https_addresses() {
        let address = WebAddress::parse(" https://www.google.com/search?q=rust ").unwrap();

        assert_eq!(address.as_str(), "https://www.google.com/search?q=rust");
        assert!(WebAddress::parse("HTTP://example.com").is_ok());
    }

    #[test]
    fn rejects_other_schemes_missing_hosts_and_spaces() {
        for invalid in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "zoommtg://join",
            "example.com",
            "https://",
            "https://example.com/a b",
        ] {
            assert!(WebAddress::parse(invalid).is_err(), "{invalid}");
        }
    }
}
