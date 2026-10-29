//! Factory for the current operating system's adapters. The only place that knows which
//! concrete adapter implements each port.

use std::sync::Arc;

use deskwright_core::ports::{
    AccessibilityPermission, AppDirectory, AppLauncher, AppTerminator, Clipboard, ElementActions,
    ElementTreeReader, HitTester, InputSynthesizer, MenuBarLocator, ScreenCapturer, ScriptRunner,
    WindowController,
};

use crate::configuration::DeskwrightConfiguration;

/// One implementation of every port, ready to inject into the core services.
pub struct PlatformAdapters {
    pub permission: Arc<dyn AccessibilityPermission>,
    pub apps: Arc<dyn AppDirectory>,
    pub launcher: Arc<dyn AppLauncher>,
    pub terminator: Arc<dyn AppTerminator>,
    pub tree: Arc<dyn ElementTreeReader>,
    pub actions: Arc<dyn ElementActions>,
    pub windows: Arc<dyn WindowController>,
    pub menu_bars: Arc<dyn MenuBarLocator>,
    pub hit_tester: Arc<dyn HitTester>,
    pub input: Arc<dyn InputSynthesizer>,
    pub capturer: Arc<dyn ScreenCapturer>,
    pub clipboard: Arc<dyn Clipboard>,
    pub scripts: Arc<dyn ScriptRunner>,
}

#[cfg(target_os = "macos")]
pub fn native_adapters(
    configuration: &DeskwrightConfiguration,
) -> anyhow::Result<PlatformAdapters> {
    use deskwright_macos::{
        AxAccessibilityPermission, AxElementActions, AxHitTester, AxMenuBarLocator, AxTreeReader,
        AxWindowController, OpenCommandAppLauncher, OsaScriptRunner, PasteboardClipboard,
        RunningAppTerminator, WorkspaceAppDirectory, configure_accessibility_timeout,
    };
    use deskwright_portable::{EnigoInputSynthesizer, XcapScreenCapturer};

    if let Err(error) = configure_accessibility_timeout(configuration.accessibility_timeout()) {
        tracing::warn!(%error, "keeping the system's default Accessibility timeout");
    }
    Ok(PlatformAdapters {
        permission: Arc::new(AxAccessibilityPermission::new()),
        apps: Arc::new(WorkspaceAppDirectory::new()),
        launcher: Arc::new(OpenCommandAppLauncher::new()),
        terminator: Arc::new(RunningAppTerminator::new()),
        tree: Arc::new(AxTreeReader::new()),
        actions: Arc::new(AxElementActions::new()),
        windows: Arc::new(AxWindowController::new()),
        menu_bars: Arc::new(AxMenuBarLocator::new()),
        hit_tester: Arc::new(AxHitTester::new()),
        input: Arc::new(EnigoInputSynthesizer::new()),
        capturer: Arc::new(XcapScreenCapturer::new()),
        clipboard: Arc::new(PasteboardClipboard::new()),
        scripts: Arc::new(OsaScriptRunner::new()),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn native_adapters(
    _configuration: &DeskwrightConfiguration,
) -> anyhow::Result<PlatformAdapters> {
    anyhow::bail!("deskwright currently supports only macOS")
}

/// Every port served by one fake desktop, for tests of the wiring.
#[cfg(test)]
pub fn fake_adapters(desktop: Arc<deskwright_core::testing::FakeDesktop>) -> PlatformAdapters {
    PlatformAdapters {
        permission: desktop.clone(),
        apps: desktop.clone(),
        launcher: desktop.clone(),
        terminator: desktop.clone(),
        tree: desktop.clone(),
        actions: desktop.clone(),
        windows: desktop.clone(),
        menu_bars: desktop.clone(),
        hit_tester: desktop.clone(),
        input: desktop.clone(),
        capturer: desktop.clone(),
        clipboard: desktop.clone(),
        scripts: desktop,
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn builds_the_macos_adapters_without_needing_permissions() {
        let configuration = DeskwrightConfiguration::parse_from(["deskwright"]);

        let adapters = native_adapters(&configuration).unwrap();

        assert!(adapters.apps.running_apps().is_ok());
    }
}
