use std::time::Duration;

use crate::error::{DeskwrightError, DeskwrightResult};

const MAX_SCRIPT_SOURCE_CHARACTERS: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptLanguage {
    AppleScript,
    JavaScript,
}

/// A validated automation script with the time it may run before being stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    language: ScriptLanguage,
    source: String,
    timeout: Duration,
}

impl Script {
    pub fn new(
        language: ScriptLanguage,
        source: &str,
        timeout: Duration,
    ) -> DeskwrightResult<Self> {
        if source.trim().is_empty() {
            return Err(DeskwrightError::InvalidInput(
                "script source is empty".to_owned(),
            ));
        }
        if source.chars().count() > MAX_SCRIPT_SOURCE_CHARACTERS {
            return Err(DeskwrightError::InvalidInput(format!(
                "script source is longer than {MAX_SCRIPT_SOURCE_CHARACTERS} characters"
            )));
        }
        Ok(Self {
            language,
            source: source.to_owned(),
            timeout,
        })
    }

    pub fn language(&self) -> ScriptLanguage {
        self.language
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMEOUT: Duration = Duration::from_secs(5);

    #[test]
    fn keeps_a_valid_script_as_given() {
        let script = Script::new(ScriptLanguage::JavaScript, "1 + 1", TIMEOUT).unwrap();

        assert_eq!(script.language(), ScriptLanguage::JavaScript);
        assert_eq!(script.source(), "1 + 1");
        assert_eq!(script.timeout(), TIMEOUT);
    }

    #[test]
    fn rejects_blank_source() {
        assert!(Script::new(ScriptLanguage::AppleScript, " \n\t", TIMEOUT).is_err());
    }

    #[test]
    fn rejects_source_over_the_length_limit() {
        let oversized = "x".repeat(MAX_SCRIPT_SOURCE_CHARACTERS + 1);

        assert!(Script::new(ScriptLanguage::AppleScript, &oversized, TIMEOUT).is_err());
    }
}
