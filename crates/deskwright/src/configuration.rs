use std::time::Duration;

use clap::Parser;

const DEFAULT_SCRIPT_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_ACCESSIBILITY_TIMEOUT_SECONDS: u64 = 3;

/// Settings shared by every command: which tools exist and how long requests may take.
#[derive(Debug, Clone, Parser)]
pub struct DeskwrightConfiguration {
    /// Register the desk_script tool, which runs arbitrary AppleScript/JXA as the user.
    #[arg(
        long,
        global = true,
        env = "DESKWRIGHT_ENABLE_SCRIPT",
        default_value_t = false
    )]
    pub enable_script: bool,

    /// Seconds a script may run before it is stopped.
    #[arg(
        long,
        global = true,
        env = "DESKWRIGHT_SCRIPT_TIMEOUT_SECONDS",
        default_value_t = DEFAULT_SCRIPT_TIMEOUT_SECONDS,
        value_parser = clap::value_parser!(u64).range(1..=600)
    )]
    pub script_timeout_seconds: u64,

    /// Seconds to wait for an unresponsive app before an Accessibility request fails.
    #[arg(
        long,
        global = true,
        env = "DESKWRIGHT_ACCESSIBILITY_TIMEOUT_SECONDS",
        default_value_t = DEFAULT_ACCESSIBILITY_TIMEOUT_SECONDS,
        value_parser = clap::value_parser!(u64).range(1..=60)
    )]
    pub accessibility_timeout_seconds: u64,
}

impl Default for DeskwrightConfiguration {
    /// The same settings as running the binary with no flags or environment variables set.
    fn default() -> Self {
        Self {
            enable_script: false,
            script_timeout_seconds: DEFAULT_SCRIPT_TIMEOUT_SECONDS,
            accessibility_timeout_seconds: DEFAULT_ACCESSIBILITY_TIMEOUT_SECONDS,
        }
    }
}

impl DeskwrightConfiguration {
    pub fn script_timeout(&self) -> Duration {
        Duration::from_secs(self.script_timeout_seconds)
    }

    pub fn accessibility_timeout(&self) -> Duration {
        Duration::from_secs(self.accessibility_timeout_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<DeskwrightConfiguration, clap::Error> {
        DeskwrightConfiguration::try_parse_from(
            std::iter::once("deskwright").chain(arguments.iter().copied()),
        )
    }

    #[test]
    fn defaults_keep_scripts_off_with_safe_timeouts() {
        let configuration = parse(&[]).unwrap();

        assert!(!configuration.enable_script);
        assert_eq!(configuration.script_timeout(), Duration::from_secs(30));
        assert_eq!(
            configuration.accessibility_timeout(),
            Duration::from_secs(3)
        );
    }

    #[test]
    fn the_default_matches_parsing_no_flags() {
        let parsed = parse(&[]).unwrap();
        let default = DeskwrightConfiguration::default();

        assert_eq!(default.enable_script, parsed.enable_script);
        assert_eq!(default.script_timeout(), parsed.script_timeout());
        assert_eq!(
            default.accessibility_timeout(),
            parsed.accessibility_timeout()
        );
    }

    #[test]
    fn accepts_explicit_values() {
        let configuration = parse(&[
            "--enable-script",
            "--script-timeout-seconds",
            "120",
            "--accessibility-timeout-seconds",
            "10",
        ])
        .unwrap();

        assert!(configuration.enable_script);
        assert_eq!(configuration.script_timeout(), Duration::from_secs(120));
        assert_eq!(
            configuration.accessibility_timeout(),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn rejects_timeouts_outside_their_ranges() {
        assert!(parse(&["--script-timeout-seconds", "0"]).is_err());
        assert!(parse(&["--script-timeout-seconds", "601"]).is_err());
        assert!(parse(&["--accessibility-timeout-seconds", "61"]).is_err());
    }
}
