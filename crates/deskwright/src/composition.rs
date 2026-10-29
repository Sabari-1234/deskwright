//! Composition root: builds the core services from the platform adapters and registers the
//! MCP tools. Nothing else in the workspace constructs services or tools.

use std::sync::Arc;
use std::time::Duration;

use deskwright_core::element_registry::ElementRegistry;
use deskwright_core::polling::PollSchedule;
use deskwright_core::services::app_launch_service::AppLaunchService;
use deskwright_core::services::app_quit_service::AppQuitService;
use deskwright_core::services::app_resolver::AppResolver;
use deskwright_core::services::element_locator_service::ElementLocatorService;
use deskwright_core::services::element_read_service::ElementReadService;
use deskwright_core::services::element_search_service::ElementSearchService;
use deskwright_core::services::element_wait_service::ElementWaitService;
use deskwright_core::services::expectation_service::ExpectationService;
use deskwright_core::services::interaction::click_service::{ClickService, ClickStrategies};
use deskwright_core::services::interaction::click_strategies::{
    AccessibilityMenuClick, AccessibilityPressClick, ClickStrategy, PointerClick, PointerGesture,
};
use deskwright_core::services::interaction::pointer_gesture_service::PointerGestureService;
use deskwright_core::services::interaction::pointer_target::PointerTarget;
use deskwright_core::services::interaction::scroll_service::ScrollService;
use deskwright_core::services::interaction::text_entry_service::TextEntryService;
use deskwright_core::services::interaction::text_entry_strategies::{
    AccessibilityValueEntry, KeyboardTextEntry,
};
use deskwright_core::services::interaction::toggle_service::ToggleService;
use deskwright_core::services::keyboard_service::KeyboardService;
use deskwright_core::services::locator_suggestion_service::LocatorSuggestionService;
use deskwright_core::services::menu_service::MenuService;
use deskwright_core::services::option_service::OptionService;
use deskwright_core::services::screenshot_service::ScreenshotService;
use deskwright_core::services::snapshot_service::SnapshotService;
use deskwright_core::services::window_service::WindowService;
use deskwright_mcp::tool_registry::{DuplicateToolName, ToolRegistry};
use deskwright_mcp::tools::{
    BatchTool, CheckTool, ClickTool, DragTool, ExpectTool, FindTool, HoverTool, LaunchAppTool,
    ListAppsTool, LocatorTool, MenuTool, PressKeysTool, QuitAppTool, ReadClipboardTool, ReadTool,
    ScreenshotTool, ScriptTool, ScrollIntoViewTool, SelectTool, SnapshotTool, TypeTextTool,
    WaitForTool, WindowTool, WriteClipboardTool,
};

use crate::configuration::DeskwrightConfiguration;

/// Pauses between re-checks of the UI while waiting for elements: quick at first, so fast
/// changes are seen almost at once, then every 150 ms so slow apps aren't hammered.
const ELEMENT_WAIT_BACKOFF: [Duration; 4] = [
    Duration::from_millis(20),
    Duration::from_millis(50),
    Duration::from_millis(100),
    Duration::from_millis(150),
];
/// How long keyboard typing waits for an element to report focus after asking for it.
const FOCUS_SETTLE_TIMEOUT: Duration = Duration::from_millis(500);
/// How long desk_check waits for a clicked checkbox to report its new state.
const TOGGLE_SETTLE_TIMEOUT: Duration = Duration::from_millis(500);
/// How long desk_select waits for an opened pop-up to list its options.
const OPTION_MENU_TIMEOUT: Duration = Duration::from_secs(2);
/// How long minimizing or restoring a window may animate before it counts as refused.
const WINDOW_SETTLE_TIMEOUT: Duration = Duration::from_secs(1);
/// How long desk_quit_app waits for an app to exit after asking it to quit.
const APP_EXIT_TIMEOUT: Duration = Duration::from_secs(5);

use crate::platform::PlatformAdapters;

pub fn build_tool_registry(
    configuration: &DeskwrightConfiguration,
    adapters: PlatformAdapters,
) -> Result<ToolRegistry, DuplicateToolName> {
    let element_registry = Arc::new(ElementRegistry::default());
    let app_resolver = AppResolver::new(adapters.apps.clone());

    let snapshots = Arc::new(SnapshotService::new(
        app_resolver.clone(),
        adapters.permission.clone(),
        adapters.tree.clone(),
        element_registry.clone(),
    ));
    let pointer_target = PointerTarget::new(adapters.tree.clone(), adapters.hit_tester.clone());
    let pointer_click = |gesture| -> Box<dyn ClickStrategy> {
        Box::new(PointerClick::new(
            pointer_target.clone(),
            adapters.input.clone(),
            gesture,
        ))
    };
    let clicks = Arc::new(ClickService::new(
        element_registry.clone(),
        ClickStrategies {
            primary: vec![
                Box::new(AccessibilityPressClick::new(adapters.actions.clone())),
                pointer_click(PointerGesture::Single),
            ],
            double: vec![pointer_click(PointerGesture::Double)],
            secondary: vec![
                Box::new(AccessibilityMenuClick::new(adapters.actions.clone())),
                pointer_click(PointerGesture::Secondary),
            ],
        },
    ));
    let toggles = Arc::new(ToggleService::new(
        element_registry.clone(),
        adapters.tree.clone(),
        clicks.clone(),
        TOGGLE_SETTLE_TIMEOUT,
    ));
    let options = Arc::new(OptionService::new(
        element_registry.clone(),
        AppResolver::new(adapters.apps.clone()),
        adapters.tree.clone(),
        adapters.actions.clone(),
        OPTION_MENU_TIMEOUT,
    ));
    let scrolls = Arc::new(ScrollService::new(
        element_registry.clone(),
        adapters.actions.clone(),
        adapters.tree.clone(),
    ));
    let text_entry = Arc::new(TextEntryService::new(
        element_registry.clone(),
        vec![
            Box::new(AccessibilityValueEntry::new(
                adapters.actions.clone(),
                adapters.tree.clone(),
            )),
            Box::new(KeyboardTextEntry::new(
                adapters.actions.clone(),
                adapters.tree.clone(),
                adapters.input.clone(),
                FOCUS_SETTLE_TIMEOUT,
            )),
        ],
    ));
    let menus = Arc::new(MenuService::new(
        app_resolver,
        adapters.permission.clone(),
        adapters.menu_bars.clone(),
        adapters.tree.clone(),
        adapters.actions.clone(),
    ));
    let reader = Arc::new(ElementReadService::new(
        element_registry.clone(),
        adapters.tree.clone(),
    ));

    let launches = Arc::new(AppLaunchService::new(
        adapters.launcher.clone(),
        AppResolver::new(adapters.apps.clone()),
    ));
    let quits = Arc::new(AppQuitService::new(
        AppResolver::new(adapters.apps.clone()),
        adapters.apps.clone(),
        adapters.terminator.clone(),
        APP_EXIT_TIMEOUT,
    ));
    let windows = Arc::new(WindowService::new(
        AppResolver::new(adapters.apps.clone()),
        adapters.permission.clone(),
        adapters.tree.clone(),
        adapters.windows.clone(),
        adapters.launcher.clone(),
        WINDOW_SETTLE_TIMEOUT,
    ));
    let screenshots = Arc::new(ScreenshotService::new(
        adapters.capturer.clone(),
        element_registry.clone(),
        adapters.tree.clone(),
        windows.clone(),
    ));
    let keyboard = Arc::new(KeyboardService::new(
        AppResolver::new(adapters.apps.clone()),
        adapters.input.clone(),
    ));

    let search = Arc::new(ElementSearchService::new(snapshots.clone()));
    let waits = Arc::new(ElementWaitService::new(
        search.clone(),
        PollSchedule::backoff(&ELEMENT_WAIT_BACKOFF),
    ));
    let locator = Arc::new(ElementLocatorService::new(waits.clone()));
    let suggestions = Arc::new(LocatorSuggestionService::new(
        element_registry.clone(),
        adapters.tree.clone(),
        search.clone(),
    ));
    let expectations = Arc::new(ExpectationService::new(
        search.clone(),
        PollSchedule::backoff(&ELEMENT_WAIT_BACKOFF),
    ));
    let gestures = Arc::new(PointerGestureService::new(
        element_registry,
        locator.clone(),
        pointer_target,
        adapters.input.clone(),
    ));

    let mut registry = ToolRegistry::new();
    registry
        .register(SnapshotTool::new(snapshots))?
        .register(FindTool::new(search))?
        .register(WaitForTool::new(waits, expectations.clone()))?
        .register(ReadTool::new(locator.clone(), reader))?
        .register(ExpectTool::new(expectations))?
        .register(LocatorTool::new(suggestions))?
        .register(ClickTool::new(locator.clone(), clicks))?
        .register(HoverTool::new(locator.clone(), gestures.clone()))?
        .register(DragTool::new(gestures))?
        .register(CheckTool::new(locator.clone(), toggles))?
        .register(SelectTool::new(locator.clone(), options))?
        .register(ScrollIntoViewTool::new(locator.clone(), scrolls))?
        .register(TypeTextTool::new(locator, text_entry, keyboard.clone()))?
        .register(MenuTool::new(menus))?
        .register(ScreenshotTool::new(screenshots))?
        .register(ListAppsTool::new(adapters.apps.clone()))?
        .register(LaunchAppTool::new(launches))?
        .register(QuitAppTool::new(quits))?
        .register(WindowTool::new(windows.clone()))?
        .register(ReadClipboardTool::new(adapters.clipboard.clone()))?
        .register(WriteClipboardTool::new(adapters.clipboard.clone()))?
        .register(PressKeysTool::new(keyboard))?
        .register(BatchTool)?;
    if configuration.enable_script {
        registry.register(ScriptTool::new(
            adapters.scripts.clone(),
            configuration.script_timeout(),
        ))?;
    }
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use deskwright_core::testing::FakeDesktop;

    use super::*;

    fn registered_tool_names(arguments: &[&str]) -> Vec<&'static str> {
        let configuration = DeskwrightConfiguration::parse_from(arguments);
        build_tool_registry(
            &configuration,
            crate::platform::fake_adapters(Arc::new(FakeDesktop::new())),
        )
        .unwrap()
        .specs()
        .into_iter()
        .map(|spec| spec.name)
        .collect()
    }

    #[test]
    fn registers_every_tool_except_script_by_default() {
        assert_eq!(
            registered_tool_names(&["deskwright"]),
            [
                "desk_batch",
                "desk_check",
                "desk_click",
                "desk_drag",
                "desk_expect",
                "desk_find",
                "desk_hover",
                "desk_launch_app",
                "desk_list_apps",
                "desk_locator",
                "desk_menu",
                "desk_press_keys",
                "desk_quit_app",
                "desk_read",
                "desk_read_clipboard",
                "desk_screenshot",
                "desk_scroll_into_view",
                "desk_select",
                "desk_snapshot",
                "desk_type",
                "desk_wait_for",
                "desk_window",
                "desk_write_clipboard",
            ]
        );
    }

    #[test]
    fn registers_the_script_tool_only_when_enabled() {
        let names = registered_tool_names(&["deskwright", "--enable-script"]);
        assert!(names.contains(&"desk_script"));
    }
}
