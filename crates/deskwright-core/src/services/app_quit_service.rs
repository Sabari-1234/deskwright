use std::sync::Arc;
use std::time::Duration;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::app::{AppInfo, AppSelector};
use crate::polling::wait_until;
use crate::ports::{AppDirectory, AppTerminator};
use crate::services::app_resolver::AppResolver;

const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Quits an app, like Playwright's `browser.close()`, and waits until it has exited.
pub struct AppQuitService {
    apps: AppResolver,
    directory: Arc<dyn AppDirectory>,
    terminator: Arc<dyn AppTerminator>,
    exit_timeout: Duration,
}

impl AppQuitService {
    /// `exit_timeout` is how long to wait for the app to exit after asking.
    pub fn new(
        apps: AppResolver,
        directory: Arc<dyn AppDirectory>,
        terminator: Arc<dyn AppTerminator>,
        exit_timeout: Duration,
    ) -> Self {
        Self {
            apps,
            directory,
            terminator,
            exit_timeout,
        }
    }

    /// Quits the app named by `name_or_bundle_id`. Quitting is never aimed at "whatever is
    /// frontmost", so the name is required.
    pub fn quit(&self, name_or_bundle_id: &str, force: bool) -> DeskwrightResult<AppInfo> {
        let name_or_bundle_id = name_or_bundle_id.trim();
        if name_or_bundle_id.is_empty() {
            return Err(DeskwrightError::InvalidInput(
                "give the name or bundle ID of the app to quit".to_owned(),
            ));
        }
        let app = self
            .apps
            .resolve(&AppSelector::NameOrBundleId(name_or_bundle_id.to_owned()))?;
        self.terminator.quit(app.process_id, force)?;
        let has_exited = wait_until(self.exit_timeout, EXIT_POLL_INTERVAL, || self.is_gone(&app));
        if has_exited {
            Ok(app)
        } else {
            Err(DeskwrightError::operation_failed(
                "quit the app",
                format!(
                    "{} is still running; it may be asking to save changes (answer its dialog, \
                     or quit with force to discard them)",
                    app.name
                ),
            ))
        }
    }

    fn is_gone(&self, app: &AppInfo) -> bool {
        self.directory.running_apps().is_ok_and(|running| {
            running
                .iter()
                .all(|running_app| running_app.process_id != app.process_id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDesktop, RecordedAction};

    fn quit_service(desktop: &Arc<FakeDesktop>) -> AppQuitService {
        AppQuitService::new(
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            Duration::ZERO,
        )
    }

    #[test]
    fn quits_the_app_and_waits_for_it_to_exit() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("TextEdit", Some("com.apple.TextEdit"));

        let app = quit_service(&desktop).quit(" textedit ", false).unwrap();

        assert_eq!(app.name, "TextEdit");
        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::QuitApp {
                process_id,
                force: false
            }]
        );
        assert!(desktop.running_apps().unwrap().is_empty());
    }

    #[test]
    fn explains_an_app_that_did_not_exit() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("TextEdit", None);
        desktop.make_ignore_quit(process_id);

        let error = quit_service(&desktop).quit("TextEdit", false).unwrap_err();

        assert!(error.to_string().contains("asking to save changes"));
    }

    #[test]
    fn force_quits_an_app_that_ignores_requests() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("TextEdit", None);
        desktop.make_ignore_quit(process_id);

        quit_service(&desktop).quit("TextEdit", true).unwrap();

        assert!(desktop.running_apps().unwrap().is_empty());
    }

    #[test]
    fn requires_an_app_name_and_a_running_app() {
        let desktop = Arc::new(FakeDesktop::new());
        let service = quit_service(&desktop);

        assert!(matches!(
            service.quit("  ", false),
            Err(DeskwrightError::InvalidInput(_))
        ));
        assert!(matches!(
            service.quit("Pages", false),
            Err(DeskwrightError::AppNotFound(_))
        ));
        assert!(desktop.recorded_actions().is_empty());
    }
}
