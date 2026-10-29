use std::sync::Arc;
use std::time::Duration;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::app::{AppInfo, AppSelector};
use crate::model::native_element::NativeElement;
use crate::model::window::{WindowCommand, WindowSelector, WindowSummary};
use crate::polling::wait_until;
use crate::ports::{AccessibilityPermission, AppLauncher, ElementTreeReader, WindowController};
use crate::services::app_resolver::AppResolver;

const WINDOW_ROLE: &str = "AXWindow";
const MINIMIZED_STATE_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Lists an app's windows and minimizes, restores, closes, raises, moves or resizes one.
pub struct WindowService {
    apps: AppResolver,
    permission: Arc<dyn AccessibilityPermission>,
    tree: Arc<dyn ElementTreeReader>,
    windows: Arc<dyn WindowController>,
    launcher: Arc<dyn AppLauncher>,
    settle_timeout: Duration,
}

/// A window found by its selector, with what was read about it.
struct FoundWindow {
    element: NativeElement,
    summary: WindowSummary,
}

impl WindowService {
    /// `settle_timeout` is how long minimizing or restoring may take (the animation) before
    /// it counts as refused.
    pub fn new(
        apps: AppResolver,
        permission: Arc<dyn AccessibilityPermission>,
        tree: Arc<dyn ElementTreeReader>,
        windows: Arc<dyn WindowController>,
        launcher: Arc<dyn AppLauncher>,
        settle_timeout: Duration,
    ) -> Self {
        Self {
            apps,
            permission,
            tree,
            windows,
            launcher,
            settle_timeout,
        }
    }

    /// The app's windows, frontmost first.
    pub fn list(&self, selector: &AppSelector) -> DeskwrightResult<(AppInfo, Vec<WindowSummary>)> {
        let app = self.resolve_app(selector)?;
        let summaries = self
            .read_windows(&app)?
            .into_iter()
            .map(|window| window.summary)
            .collect();
        Ok((app, summaries))
    }

    /// Runs `command` on the selected window and returns its state afterwards (before, for
    /// a closed window).
    pub fn apply(
        &self,
        app_selector: &AppSelector,
        window_selector: &WindowSelector,
        command: WindowCommand,
    ) -> DeskwrightResult<WindowSummary> {
        let app = self.resolve_app(app_selector)?;
        let window = self.select(&app, window_selector)?;
        self.run(&app, &window.element, command)?;
        if command == WindowCommand::Close {
            return Ok(window.summary);
        }
        self.summarize(window.summary.index, &window.element)
    }

    fn run(
        &self,
        app: &AppInfo,
        window: &NativeElement,
        command: WindowCommand,
    ) -> DeskwrightResult<()> {
        match command {
            WindowCommand::Minimize => self.set_minimized_verified(window, true),
            WindowCommand::Restore => self.set_minimized_verified(window, false),
            WindowCommand::Close => self.windows.close(window),
            WindowCommand::Raise => {
                if self.windows.is_minimized(window)? {
                    self.set_minimized_verified(window, false)?;
                }
                self.windows.raise(window)?;
                let app_id = app.bundle_id.as_deref().unwrap_or(&app.name);
                self.launcher.open(app_id, None)
            }
            WindowCommand::MoveTo(origin) => self.windows.move_to(window, origin),
            WindowCommand::Resize(size) => self.windows.resize(window, size),
        }
    }

    /// Apps report success for minimize requests they ignore (for example with Stage
    /// Manager on), so the state is read back until it changes.
    fn set_minimized_verified(
        &self,
        window: &NativeElement,
        should_minimize: bool,
    ) -> DeskwrightResult<()> {
        self.windows.set_minimized(window, should_minimize)?;
        let has_changed = wait_until(self.settle_timeout, MINIMIZED_STATE_POLL_INTERVAL, || {
            self.windows.is_minimized(window).ok() == Some(should_minimize)
        });
        if has_changed {
            Ok(())
        } else {
            Err(DeskwrightError::operation_failed(
                if should_minimize {
                    "minimize the window"
                } else {
                    "restore the window"
                },
                "macOS accepted the request but the window did not change; Stage Manager, \
                 full-screen windows and some apps don't allow it",
            ))
        }
    }

    fn resolve_app(&self, selector: &AppSelector) -> DeskwrightResult<AppInfo> {
        self.permission.ensure_granted()?;
        self.apps.resolve(selector)
    }

    fn select(&self, app: &AppInfo, selector: &WindowSelector) -> DeskwrightResult<FoundWindow> {
        let index_among_matches = selector.index.unwrap_or(0);
        self.read_windows(app)?
            .into_iter()
            .filter(|window| selector.matches_title(window.summary.title.as_deref()))
            .nth(index_among_matches)
            .ok_or_else(|| {
                DeskwrightError::operation_failed(
                    "find the window",
                    format!("{} has no {selector}", app.name),
                )
            })
    }

    fn read_windows(&self, app: &AppInfo) -> DeskwrightResult<Vec<FoundWindow>> {
        let mut windows = Vec::new();
        for element in self.tree.top_level_elements(app.process_id)? {
            if self.tree.attributes(&element)?.role != WINDOW_ROLE {
                continue;
            }
            let summary = self.summarize(windows.len(), &element)?;
            windows.push(FoundWindow { element, summary });
        }
        Ok(windows)
    }

    fn summarize(&self, index: usize, window: &NativeElement) -> DeskwrightResult<WindowSummary> {
        let attributes = self.tree.attributes(window)?;
        Ok(WindowSummary {
            index,
            title: attributes.title,
            bounds: attributes.bounds,
            is_minimized: self.windows.is_minimized(window)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Permission;
    use crate::model::app::ProcessId;
    use crate::model::geometry::{Bounds, Point, Size};
    use crate::testing::{FakeDesktop, FakeElementId, RecordedAction, element};

    fn window_service(desktop: &Arc<FakeDesktop>) -> WindowService {
        WindowService::new(
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            desktop.clone(),
            desktop.clone(),
            Duration::ZERO,
        )
    }

    fn pages_with_two_documents(
        desktop: &FakeDesktop,
    ) -> (ProcessId, FakeElementId, FakeElementId) {
        let process_id = desktop.add_app("Pages", Some("com.apple.iWork.Pages"));
        desktop.make_frontmost(process_id);
        let mut report = element("AXWindow", Some("Report.pages"));
        report.bounds = Some(Bounds {
            x: 10.0,
            y: 20.0,
            width: 800.0,
            height: 600.0,
        });
        let report = desktop.add_window(process_id, report);
        let notes = desktop.add_window(process_id, element("AXWindow", Some("Notes.pages")));
        desktop.add_window(process_id, element("AXSheet", Some("Not a window")));
        (process_id, report, notes)
    }

    fn titled(title: &str) -> WindowSelector {
        WindowSelector {
            title: Some(title.to_owned()),
            index: None,
        }
    }

    #[test]
    fn lists_only_windows_frontmost_first() {
        let desktop = Arc::new(FakeDesktop::new());
        pages_with_two_documents(&desktop);

        let (app, windows) = window_service(&desktop)
            .list(&AppSelector::Frontmost)
            .unwrap();

        assert_eq!(app.name, "Pages");
        let titles: Vec<_> = windows
            .iter()
            .map(|window| window.title.as_deref())
            .collect();
        assert_eq!(titles, [Some("Report.pages"), Some("Notes.pages")]);
        assert_eq!(windows[1].index, 1);
    }

    #[test]
    fn minimizes_and_restores_the_selected_window() {
        let desktop = Arc::new(FakeDesktop::new());
        let (_, _, notes) = pages_with_two_documents(&desktop);
        let service = window_service(&desktop);

        let minimized = service
            .apply(
                &AppSelector::Frontmost,
                &titled("notes"),
                WindowCommand::Minimize,
            )
            .unwrap();
        let restored = service
            .apply(
                &AppSelector::Frontmost,
                &titled("notes"),
                WindowCommand::Restore,
            )
            .unwrap();

        assert!(minimized.is_minimized);
        assert!(!restored.is_minimized);
        assert_eq!(
            desktop.recorded_actions(),
            [
                RecordedAction::SetMinimized(notes, true),
                RecordedAction::SetMinimized(notes, false)
            ]
        );
    }

    #[test]
    fn moves_and_resizes_the_frontmost_window_by_default() {
        let desktop = Arc::new(FakeDesktop::new());
        pages_with_two_documents(&desktop);
        let service = window_service(&desktop);
        let frontmost = WindowSelector::default();

        service
            .apply(
                &AppSelector::Frontmost,
                &frontmost,
                WindowCommand::MoveTo(Point { x: 0.0, y: 25.0 }),
            )
            .unwrap();
        let resized = service
            .apply(
                &AppSelector::Frontmost,
                &frontmost,
                WindowCommand::Resize(Size {
                    width: 1200.0,
                    height: 900.0,
                }),
            )
            .unwrap();

        assert_eq!(
            resized.bounds,
            Some(Bounds {
                x: 0.0,
                y: 25.0,
                width: 1200.0,
                height: 900.0
            })
        );
    }

    #[test]
    fn raising_restores_the_window_and_activates_its_app() {
        let desktop = Arc::new(FakeDesktop::new());
        let (_, _, notes) = pages_with_two_documents(&desktop);
        let selector = WindowSelector {
            title: None,
            index: Some(1),
        };

        window_service(&desktop)
            .apply(&AppSelector::Frontmost, &selector, WindowCommand::Raise)
            .unwrap();

        assert_eq!(
            desktop.recorded_actions(),
            [
                RecordedAction::Raise(notes),
                RecordedAction::OpenApp {
                    name_or_bundle_id: "com.apple.iWork.Pages".to_owned(),
                    address: None
                }
            ]
        );
    }

    #[test]
    fn closing_reports_the_window_as_it_was() {
        let desktop = Arc::new(FakeDesktop::new());
        let (_, report, _) = pages_with_two_documents(&desktop);
        let service = window_service(&desktop);

        let closed = service
            .apply(
                &AppSelector::Frontmost,
                &titled("report"),
                WindowCommand::Close,
            )
            .unwrap();

        assert_eq!(closed.title.as_deref(), Some("Report.pages"));
        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::CloseWindow(report)]
        );
        let (_, remaining) = service.list(&AppSelector::Frontmost).unwrap();
        assert_eq!(remaining.len(), 1);
    }

    #[test]
    fn names_the_selection_when_no_window_matches() {
        let desktop = Arc::new(FakeDesktop::new());
        pages_with_two_documents(&desktop);

        let error = window_service(&desktop)
            .apply(
                &AppSelector::Frontmost,
                &titled("budget"),
                WindowCommand::Close,
            )
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("Pages has no window titled \"budget\"")
        );
        assert!(desktop.recorded_actions().is_empty());
    }

    #[test]
    fn requires_accessibility_permission() {
        let desktop = Arc::new(FakeDesktop::new());
        pages_with_two_documents(&desktop);
        desktop.deny_accessibility_permission();

        let result = window_service(&desktop).list(&AppSelector::Frontmost);

        assert_eq!(
            result.unwrap_err(),
            DeskwrightError::PermissionDenied {
                permission: Permission::Accessibility
            }
        );
    }

    #[test]
    fn reports_a_minimize_request_the_window_ignored() {
        let desktop = Arc::new(FakeDesktop::new());
        let (_, report, _) = pages_with_two_documents(&desktop);
        desktop.make_minimize_ignored(report);

        let error = window_service(&desktop)
            .apply(
                &AppSelector::Frontmost,
                &titled("report"),
                WindowCommand::Minimize,
            )
            .unwrap_err();

        assert!(error.to_string().contains("the window did not change"));
    }

    #[test]
    fn raising_a_minimized_window_restores_it_first() {
        let desktop = Arc::new(FakeDesktop::new());
        let (_, report, _) = pages_with_two_documents(&desktop);
        let service = window_service(&desktop);
        service
            .apply(
                &AppSelector::Frontmost,
                &titled("report"),
                WindowCommand::Minimize,
            )
            .unwrap();

        let raised = service
            .apply(
                &AppSelector::Frontmost,
                &titled("report"),
                WindowCommand::Raise,
            )
            .unwrap();

        assert!(!raised.is_minimized);
        assert_eq!(
            desktop.recorded_actions()[1..3],
            [
                RecordedAction::SetMinimized(report, false),
                RecordedAction::Raise(report)
            ]
        );
    }
}
