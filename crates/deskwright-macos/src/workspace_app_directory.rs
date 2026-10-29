use deskwright_core::DeskwrightResult;
use deskwright_core::model::app::{AppInfo, ProcessId};
use deskwright_core::ports::AppDirectory;
use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication};

use crate::process_list::all_process_ids;

/// [`AppDirectory`] listing the apps that appear in the Dock.
///
/// `NSWorkspace.runningApplications` only refreshes while a main run loop runs, which this
/// server doesn't have, so apps launched after startup would be missing. Instead every call
/// reads the live process list from the kernel and asks a freshly created
/// `NSRunningApplication` about each process, including whether it is the active app.
#[derive(Debug, Default)]
pub struct WorkspaceAppDirectory;

impl WorkspaceAppDirectory {
    pub fn new() -> Self {
        Self
    }
}

impl AppDirectory for WorkspaceAppDirectory {
    fn running_apps(&self) -> DeskwrightResult<Vec<AppInfo>> {
        let process_ids = all_process_ids()?;
        Ok(autoreleasepool(|_| {
            process_ids
                .into_iter()
                .filter_map(|process_id| {
                    NSRunningApplication::runningApplicationWithProcessIdentifier(process_id.0)
                })
                .filter(|app| app.activationPolicy() == NSApplicationActivationPolicy::Regular)
                .map(|app| app_info(&app))
                .collect()
        }))
    }

    fn frontmost_app(&self) -> DeskwrightResult<Option<AppInfo>> {
        Ok(self.running_apps()?.into_iter().find(|app| app.is_active))
    }
}

fn app_info(app: &NSRunningApplication) -> AppInfo {
    let process_id = ProcessId(app.processIdentifier());
    AppInfo {
        name: app
            .localizedName()
            .map(|name| name.to_string())
            .unwrap_or_default(),
        bundle_id: app
            .bundleIdentifier()
            .map(|bundle_id| bundle_id.to_string()),
        process_id,
        is_active: app.isActive(),
    }
}

// Reads the real running-app list, which needs no permissions.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_apps_with_names_and_valid_process_ids() {
        let apps = WorkspaceAppDirectory::new().running_apps().unwrap();

        assert!(apps.iter().all(|app| app.process_id.0 > 0));
        assert!(apps.iter().all(|app| !app.name.is_empty()));
        assert!(apps.iter().filter(|app| app.is_active).count() <= 1);
    }

    #[test]
    fn frontmost_app_is_one_of_the_running_apps() {
        let directory = WorkspaceAppDirectory::new();

        let frontmost = directory.frontmost_app().unwrap();

        if let Some(frontmost) = frontmost {
            let running = directory.running_apps().unwrap();
            assert!(
                running
                    .iter()
                    .any(|app| app.process_id == frontmost.process_id)
            );
        }
    }
}
