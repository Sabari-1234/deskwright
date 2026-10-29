use std::sync::Arc;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::app::{AppInfo, AppSelector};
use crate::model::web_address::WebAddress;
use crate::ports::AppLauncher;
use crate::services::app_resolver::AppResolver;

/// Launches or activates an app (optionally opening a web address in it) and reports the app
/// that is now frontmost.
pub struct AppLaunchService {
    launcher: Arc<dyn AppLauncher>,
    apps: AppResolver,
}

impl AppLaunchService {
    pub fn new(launcher: Arc<dyn AppLauncher>, apps: AppResolver) -> Self {
        Self { launcher, apps }
    }

    pub fn open(
        &self,
        name_or_bundle_id: &str,
        address: Option<&WebAddress>,
    ) -> DeskwrightResult<AppInfo> {
        let name_or_bundle_id = name_or_bundle_id.trim();
        if name_or_bundle_id.is_empty() {
            return Err(DeskwrightError::InvalidInput(
                "give the name or bundle ID of the app to open".to_owned(),
            ));
        }
        self.launcher.open(name_or_bundle_id, address)?;
        self.apps
            .resolve(&AppSelector::NameOrBundleId(name_or_bundle_id.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDesktop, RecordedAction};

    fn launch_service(desktop: &Arc<FakeDesktop>) -> AppLaunchService {
        AppLaunchService::new(desktop.clone(), AppResolver::new(desktop.clone()))
    }

    #[test]
    fn brings_a_running_app_to_the_front() {
        let desktop = Arc::new(FakeDesktop::new());
        desktop.add_app("Notes", None);
        let finder = desktop.add_app("Finder", Some("com.apple.finder"));
        desktop.make_frontmost(finder);

        let app = launch_service(&desktop).open(" Notes ", None).unwrap();

        assert_eq!(app.name, "Notes");
        assert!(app.is_active);
    }

    #[test]
    fn launches_an_app_that_is_not_running_and_opens_the_address() {
        let desktop = Arc::new(FakeDesktop::new());
        let address = WebAddress::parse("https://example.com").unwrap();

        let app = launch_service(&desktop)
            .open("Google Chrome", Some(&address))
            .unwrap();

        assert!(app.is_active);
        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::OpenApp {
                name_or_bundle_id: "Google Chrome".to_owned(),
                address: Some("https://example.com".to_owned()),
            }]
        );
    }

    #[test]
    fn rejects_a_blank_app_name() {
        let desktop = Arc::new(FakeDesktop::new());

        assert!(launch_service(&desktop).open("  ", None).is_err());
        assert!(desktop.recorded_actions().is_empty());
    }

    #[test]
    fn reports_apps_that_are_not_installed() {
        let desktop = Arc::new(FakeDesktop::new());
        desktop.make_not_installed("Ghost");

        let error = launch_service(&desktop).open("Ghost", None).unwrap_err();

        assert_eq!(error, DeskwrightError::AppNotInstalled("Ghost".to_owned()));
    }
}
