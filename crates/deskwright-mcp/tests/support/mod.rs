//! Shared fixture for the tool tests: every tool wired to one `FakeDesktop`.
// Each test binary uses a different subset of these helpers.
#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use deskwright_core::element_registry::ElementRegistry;
use deskwright_core::model::geometry::Bounds;
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
    AccessibilityMenuClick, AccessibilityPressClick, PointerClick, PointerGesture,
};
use deskwright_core::services::interaction::pointer_gesture_service::PointerGestureService;
use deskwright_core::services::interaction::pointer_target::PointerTarget;
use deskwright_core::services::interaction::scroll_service::ScrollService;
use deskwright_core::services::interaction::text_entry_service::TextEntryService;
use deskwright_core::services::interaction::text_entry_strategies::AccessibilityValueEntry;
use deskwright_core::services::interaction::toggle_service::ToggleService;
use deskwright_core::services::keyboard_service::KeyboardService;
use deskwright_core::services::locator_suggestion_service::LocatorSuggestionService;
use deskwright_core::services::menu_service::MenuService;
use deskwright_core::services::option_service::OptionService;
use deskwright_core::services::screenshot_service::ScreenshotService;
use deskwright_core::services::snapshot_service::SnapshotService;
use deskwright_core::services::window_service::WindowService;
use deskwright_core::testing::{FakeDesktop, FakeElementId, element};
use deskwright_mcp::tool::{JsonObject, OutputBlock, ToolDispatcher, ToolOutput};
use deskwright_mcp::tool_registry::ToolRegistry;
use deskwright_mcp::tools::{
    BatchTool, CheckTool, ClickTool, DragTool, ExpectTool, FindTool, HoverTool, LaunchAppTool,
    ListAppsTool, LocatorTool, MenuTool, PressKeysTool, QuitAppTool, ReadClipboardTool, ReadTool,
    ScreenshotTool, ScriptTool, ScrollIntoViewTool, SelectTool, SnapshotTool, TypeTextTool,
    WaitForTool, WindowTool, WriteClipboardTool,
};

pub const SCRIPT_TIMEOUT: Duration = Duration::from_secs(5);

/// Refs assigned by a snapshot of the fixture's frontmost app, in depth-first order.
pub const WINDOW_REF: &str = "e1";
pub const SEND_BUTTON_REF: &str = "e2";
pub const MESSAGE_FIELD_REF: &str = "e3";

/// The fake desktop: "Notes" (frontmost) with a window holding a Send button and a Message
/// field, plus a File > Export… menu.
pub struct Fixture {
    pub desktop: Arc<FakeDesktop>,
    pub registry: ToolRegistry,
    pub export_menu_item: FakeElementId,
}

impl Fixture {
    pub fn new() -> Self {
        let desktop = Arc::new(FakeDesktop::new());
        let export_menu_item = populate_desktop(&desktop);
        let registry = register_all_tools(&desktop);
        Self {
            desktop,
            registry,
            export_menu_item,
        }
    }

    pub async fn call(&self, tool_name: &str, arguments: serde_json::Value) -> ToolOutput {
        let arguments: JsonObject =
            serde_json::from_value(arguments).expect("arguments are an object");
        self.registry.dispatch(tool_name, arguments).await
    }
}

/// All text blocks of an output, joined by newlines.
pub fn text_of(output: &ToolOutput) -> String {
    output
        .blocks
        .iter()
        .filter_map(|block| match block {
            OutputBlock::Text(text) => Some(text.as_str()),
            OutputBlock::PngImage { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn populate_desktop(desktop: &FakeDesktop) -> FakeElementId {
    let process_id = desktop.add_app("Notes", Some("com.apple.Notes"));
    desktop.make_frontmost(process_id);
    let window = desktop.add_window(process_id, element("AXWindow", Some("Main")));
    let mut send_button = element("AXButton", Some("Send"));
    send_button.bounds = Some(Bounds {
        x: 0.0,
        y: 0.0,
        width: 40.0,
        height: 20.0,
    });
    desktop.add_child(window, send_button);
    let message_field = desktop.add_child(window, element("AXTextField", Some("Message")));
    desktop.set_value(message_field, "");

    let menu_bar = desktop.set_menu_bar(process_id, element("AXMenuBar", None));
    let file = desktop.add_child(menu_bar, element("AXMenuBarItem", Some("File")));
    let file_menu = desktop.add_child(file, element("AXMenu", None));
    desktop.add_child(file_menu, element("AXMenuItem", Some("Export…")))
}

fn register_all_tools(desktop: &Arc<FakeDesktop>) -> ToolRegistry {
    let element_registry = Arc::new(ElementRegistry::default());
    let app_resolver = AppResolver::new(desktop.clone());
    let snapshots = Arc::new(SnapshotService::new(
        app_resolver.clone(),
        desktop.clone(),
        desktop.clone(),
        element_registry.clone(),
    ));
    let pointer_target = PointerTarget::new(desktop.clone(), desktop.clone());
    let clicks = Arc::new(ClickService::new(
        element_registry.clone(),
        ClickStrategies {
            primary: vec![Box::new(AccessibilityPressClick::new(desktop.clone()))],
            double: vec![Box::new(PointerClick::new(
                pointer_target.clone(),
                desktop.clone(),
                PointerGesture::Double,
            ))],
            secondary: vec![Box::new(AccessibilityMenuClick::new(desktop.clone()))],
        },
    ));
    let toggles = Arc::new(ToggleService::new(
        element_registry.clone(),
        desktop.clone(),
        clicks.clone(),
        Duration::ZERO,
    ));
    let options = Arc::new(OptionService::new(
        element_registry.clone(),
        AppResolver::new(desktop.clone()),
        desktop.clone(),
        desktop.clone(),
        Duration::ZERO,
    ));
    let scrolls = Arc::new(ScrollService::new(
        element_registry.clone(),
        desktop.clone(),
        desktop.clone(),
    ));
    let text_entry = Arc::new(TextEntryService::new(
        element_registry.clone(),
        vec![Box::new(AccessibilityValueEntry::new(
            desktop.clone(),
            desktop.clone(),
        ))],
    ));
    let menus = Arc::new(MenuService::new(
        app_resolver,
        desktop.clone(),
        desktop.clone(),
        desktop.clone(),
        desktop.clone(),
    ));
    let reader = Arc::new(ElementReadService::new(
        element_registry.clone(),
        desktop.clone(),
    ));
    let screenshots = Arc::new(ScreenshotService::new(
        desktop.clone(),
        element_registry.clone(),
        desktop.clone(),
    ));

    let launches = Arc::new(AppLaunchService::new(
        desktop.clone(),
        AppResolver::new(desktop.clone()),
    ));
    let quits = Arc::new(AppQuitService::new(
        AppResolver::new(desktop.clone()),
        desktop.clone(),
        desktop.clone(),
        Duration::ZERO,
    ));
    let windows = Arc::new(WindowService::new(
        AppResolver::new(desktop.clone()),
        desktop.clone(),
        desktop.clone(),
        desktop.clone(),
        desktop.clone(),
        Duration::ZERO,
    ));
    let keyboard = Arc::new(KeyboardService::new(
        AppResolver::new(desktop.clone()),
        desktop.clone(),
    ));

    let search = Arc::new(ElementSearchService::new(snapshots.clone()));
    let waits = Arc::new(ElementWaitService::new(
        search.clone(),
        std::time::Duration::ZERO,
    ));
    let locator = Arc::new(ElementLocatorService::new(waits.clone()));
    let suggestions = Arc::new(LocatorSuggestionService::new(
        element_registry.clone(),
        desktop.clone(),
        search.clone(),
    ));
    let expectations = Arc::new(ExpectationService::new(
        search.clone(),
        std::time::Duration::ZERO,
    ));
    let gestures = Arc::new(PointerGestureService::new(
        element_registry,
        locator.clone(),
        pointer_target,
        desktop.clone(),
    ));

    let mut registry = ToolRegistry::new();
    registry
        .register(SnapshotTool::new(snapshots))
        .and_then(|registry| registry.register(FindTool::new(search)))
        .and_then(|registry| registry.register(ReadTool::new(locator.clone(), reader)))
        .and_then(|registry| registry.register(ClickTool::new(locator.clone(), clicks)))
        .and_then(|registry| registry.register(HoverTool::new(locator.clone(), gestures.clone())))
        .and_then(|registry| registry.register(DragTool::new(gestures)))
        .and_then(|registry| registry.register(CheckTool::new(locator.clone(), toggles)))
        .and_then(|registry| registry.register(SelectTool::new(locator.clone(), options)))
        .and_then(|registry| registry.register(ScrollIntoViewTool::new(locator.clone(), scrolls)))
        .and_then(|registry| {
            registry.register(TypeTextTool::new(locator, text_entry, keyboard.clone()))
        })
        .and_then(|registry| registry.register(MenuTool::new(menus)))
        .and_then(|registry| registry.register(ScreenshotTool::new(screenshots)))
        .and_then(|registry| registry.register(ListAppsTool::new(desktop.clone())))
        .and_then(|registry| registry.register(ScriptTool::new(desktop.clone(), SCRIPT_TIMEOUT)))
        .and_then(|registry| registry.register(LaunchAppTool::new(launches)))
        .and_then(|registry| registry.register(QuitAppTool::new(quits)))
        .and_then(|registry| registry.register(WindowTool::new(windows)))
        .and_then(|registry| registry.register(ReadClipboardTool::new(desktop.clone())))
        .and_then(|registry| registry.register(WriteClipboardTool::new(desktop.clone())))
        .and_then(|registry| registry.register(PressKeysTool::new(keyboard)))
        .and_then(|registry| registry.register(WaitForTool::new(waits, expectations.clone())))
        .and_then(|registry| registry.register(ExpectTool::new(expectations)))
        .and_then(|registry| registry.register(LocatorTool::new(suggestions)))
        .and_then(|registry| registry.register(BatchTool))
        .expect("tool names are unique");
    registry
}
