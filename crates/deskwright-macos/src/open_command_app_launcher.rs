use std::process::Command;
use std::time::Duration;

use deskwright_core::model::web_address::WebAddress;
use deskwright_core::polling::wait_until;
use deskwright_core::ports::{AppDirectory, AppLauncher};
use deskwright_core::{DeskwrightError, DeskwrightResult};

use crate::workspace_app_directory::WorkspaceAppDirectory;

const OPEN_COMMAND_PATH: &str = "/usr/bin/open";
const APP_NOT_FOUND_MESSAGE: &str = "Unable to find application";
const MINIMUM_BUNDLE_ID_SEPARATORS: usize = 2;
const FRONTMOST_TIMEOUT: Duration = Duration::from_secs(10);
const FRONTMOST_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// [`AppLauncher`] that uses the system `open` command, which launches or activates an app
/// through Launch Services. Arguments are passed directly, never through a shell.
#[derive(Debug, Default)]
pub struct OpenCommandAppLauncher;

impl OpenCommandAppLauncher {
    pub fn new() -> Self {
        Self
    }
}

impl AppLauncher for OpenCommandAppLauncher {
    fn open(&self, name_or_bundle_id: &str, address: Option<&WebAddress>) -> DeskwrightResult<()> {
        let output = Command::new(OPEN_COMMAND_PATH)
            .args(open_arguments(name_or_bundle_id, address))
            .output()
            .map_err(|error| DeskwrightError::operation_failed("run open", error.to_string()))?;
        if !output.status.success() {
            return Err(open_failure(
                name_or_bundle_id,
                &String::from_utf8_lossy(&output.stderr),
            ));
        }
        let is_frontmost = wait_until(FRONTMOST_TIMEOUT, FRONTMOST_POLL_INTERVAL, || {
            is_frontmost_app(name_or_bundle_id)
        });
        if is_frontmost {
            Ok(())
        } else {
            Err(DeskwrightError::AppNotFrontmost(
                name_or_bundle_id.to_owned(),
            ))
        }
    }
}

/// `-b` for a reverse-DNS bundle ID such as `com.google.Chrome`, `-a` for a display name.
fn open_arguments(name_or_bundle_id: &str, address: Option<&WebAddress>) -> Vec<String> {
    let app_flag = if looks_like_bundle_id(name_or_bundle_id) {
        "-b"
    } else {
        "-a"
    };
    let mut arguments = vec![app_flag.to_owned(), name_or_bundle_id.to_owned()];
    arguments.extend(address.map(|address| address.as_str().to_owned()));
    arguments
}

fn looks_like_bundle_id(name_or_bundle_id: &str) -> bool {
    name_or_bundle_id.matches('.').count() >= MINIMUM_BUNDLE_ID_SEPARATORS
        && !name_or_bundle_id.contains(char::is_whitespace)
}

fn open_failure(name_or_bundle_id: &str, error_output: &str) -> DeskwrightError {
    if error_output.contains(APP_NOT_FOUND_MESSAGE) {
        DeskwrightError::AppNotInstalled(name_or_bundle_id.to_owned())
    } else {
        DeskwrightError::operation_failed("open the app", error_output.trim())
    }
}

fn is_frontmost_app(name_or_bundle_id: &str) -> bool {
    WorkspaceAppDirectory::new()
        .frontmost_app()
        .ok()
        .flatten()
        .is_some_and(|app| app.is_identified_by(name_or_bundle_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_display_names_with_the_application_flag() {
        assert_eq!(
            open_arguments("Google Chrome", None),
            ["-a", "Google Chrome"]
        );
        assert_eq!(open_arguments("TextEdit", None), ["-a", "TextEdit"]);
    }

    #[test]
    fn opens_bundle_ids_with_the_bundle_flag_and_appends_the_address() {
        let address = WebAddress::parse("https://example.com").unwrap();

        assert_eq!(
            open_arguments("com.google.Chrome", Some(&address)),
            ["-b", "com.google.Chrome", "https://example.com"]
        );
    }

    #[test]
    fn treats_dotted_names_with_spaces_as_display_names() {
        assert!(!looks_like_bundle_id("Mr. Robot v.2"));
        assert!(!looks_like_bundle_id("Node.js"));
    }

    #[test]
    fn reports_a_missing_app_as_not_installed() {
        let error = open_failure("Ghost", "Unable to find application named 'Ghost'\n");

        assert_eq!(error, DeskwrightError::AppNotInstalled("Ghost".to_owned()));
    }

    #[test]
    fn keeps_other_failures_with_their_message() {
        let error = open_failure("TextEdit", " LSOpenURLsWithRole() failed \n");

        assert_eq!(
            error.to_string(),
            "open the app failed: LSOpenURLsWithRole() failed"
        );
    }

    #[test]
    fn reports_a_missing_app_through_the_real_open_command() {
        let error = OpenCommandAppLauncher::new()
            .open("No Such App Deskwright Test", None)
            .unwrap_err();

        assert_eq!(
            error,
            DeskwrightError::AppNotInstalled("No Such App Deskwright Test".to_owned())
        );
    }
}
