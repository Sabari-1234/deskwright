use std::sync::Arc;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::app::{AppInfo, AppSelector};
use crate::ports::AppDirectory;

/// Turns an [`AppSelector`] into the running app it refers to.
#[derive(Clone)]
pub struct AppResolver {
    apps: Arc<dyn AppDirectory>,
}

impl AppResolver {
    pub fn new(apps: Arc<dyn AppDirectory>) -> Self {
        Self { apps }
    }

    pub fn resolve(&self, selector: &AppSelector) -> DeskwrightResult<AppInfo> {
        match selector {
            AppSelector::Frontmost => self
                .apps
                .frontmost_app()?
                .ok_or(DeskwrightError::NoFrontmostApp),
            AppSelector::NameOrBundleId(name_or_bundle_id) => self
                .apps
                .running_apps()?
                .into_iter()
                .find(|app| app.is_identified_by(name_or_bundle_id))
                .ok_or_else(|| DeskwrightError::AppNotFound(name_or_bundle_id.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeDesktop;

    #[test]
    fn resolves_by_name_or_bundle_id_ignoring_case() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("TextEdit", Some("com.apple.TextEdit"));
        let resolver = AppResolver::new(desktop);

        let by_name = AppSelector::from_optional_name(Some("textedit"));
        let by_bundle_id = AppSelector::from_optional_name(Some("COM.APPLE.TEXTEDIT"));

        assert_eq!(resolver.resolve(&by_name).unwrap().process_id, process_id);
        assert_eq!(
            resolver.resolve(&by_bundle_id).unwrap().process_id,
            process_id
        );
    }

    #[test]
    fn reports_missing_apps() {
        let desktop = Arc::new(FakeDesktop::new());
        let resolver = AppResolver::new(desktop);

        assert_eq!(
            resolver.resolve(&AppSelector::Frontmost).unwrap_err(),
            DeskwrightError::NoFrontmostApp
        );
        assert_eq!(
            resolver
                .resolve(&AppSelector::NameOrBundleId("Ghost".to_owned()))
                .unwrap_err(),
            DeskwrightError::AppNotFound("Ghost".to_owned())
        );
    }
}
