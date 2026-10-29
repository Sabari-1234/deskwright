use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::element_registry::ElementRegistry;
use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::app::{AppInfo, ProcessId};
use crate::model::element::{ElementAttributes, ElementRef};
use crate::model::encoded_image::{CaptureOptions, EncodedImage};
use crate::model::geometry::{Bounds, Point, Size};
use crate::model::key_chord::KeyChord;
use crate::model::native_element::NativeElement;
use crate::model::script::Script;
use crate::model::web_address::WebAddress;
use crate::ports::{
    AccessibilityPermission, AppDirectory, AppLauncher, AppTerminator, Clipboard, ElementActions,
    ElementTreeReader, HitTester, InputSynthesizer, MenuBarLocator, ScreenCapturer, ScriptRunner,
    WindowController,
};

const FIRST_FAKE_PROCESS_ID: i32 = 100;
const CHECKBOX_ROLE: &str = "AXCheckBox";
const CHECKED_VALUE: &str = "1";
const UNCHECKED_VALUE: &str = "0";
const FAKE_PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Identifies an element inside a [`FakeDesktop`]; it is the payload of its `NativeElement`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FakeElementId(usize);

/// A side effect a port call had on the fake desktop, in call order.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordedAction {
    Press(FakeElementId),
    Focus(FakeElementId),
    SetValue(FakeElementId, String),
    TypeText(String),
    ClickAt(Point),
    DoubleClickAt(Point),
    RightClickAt(Point),
    MovePointerTo(Point),
    Drag {
        from: Point,
        to: Point,
    },
    ShowMenu(FakeElementId),
    ScrollIntoView(FakeElementId),
    Cancel(FakeElementId),
    Raise(FakeElementId),
    SetMinimized(FakeElementId, bool),
    MoveWindow(FakeElementId, Point),
    ResizeWindow(FakeElementId, Size),
    CloseWindow(FakeElementId),
    QuitApp {
        process_id: ProcessId,
        force: bool,
    },
    PressChord(KeyChord),
    OpenApp {
        name_or_bundle_id: String,
        address: Option<String>,
    },
    RunScript(String),
}

/// Attributes for an enabled element with the given role and optional title.
pub fn element(role: &str, title: Option<&str>) -> ElementAttributes {
    ElementAttributes {
        role: role.to_owned(),
        title: title.map(str::to_owned),
        is_enabled: true,
        ..ElementAttributes::default()
    }
}

#[derive(Default)]
struct FakeElement {
    attributes: ElementAttributes,
    children: Vec<FakeElementId>,
    is_unreadable: bool,
    does_press_fail: bool,
    does_focus_fail: bool,
    does_show_menu_fail: bool,
    is_set_value_ignored: bool,
    is_covered_by_another_app: bool,
    is_minimized: bool,
    is_minimize_ignored: bool,
    is_scroll_unsupported: bool,
}

struct DesktopState {
    is_accessibility_granted: bool,
    apps: Vec<AppInfo>,
    elements: Vec<FakeElement>,
    top_level_by_process: HashMap<ProcessId, Vec<FakeElementId>>,
    menu_bar_by_process: HashMap<ProcessId, FakeElementId>,
    recorded_actions: Vec<RecordedAction>,
    captured_regions: Vec<Bounds>,
    captured_windows: Vec<(ProcessId, Bounds)>,
    capture_options: Vec<CaptureOptions>,
    script_result: DeskwrightResult<String>,
    not_installed_apps: Vec<String>,
    element_read_count: usize,
    apps_ignoring_quit: Vec<ProcessId>,
    clipboard_text: Option<String>,
}

/// A scriptable in-memory desktop implementing every port, for tests without OS access.
pub struct FakeDesktop {
    state: Mutex<DesktopState>,
}

impl Default for FakeDesktop {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeDesktop {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(DesktopState {
                is_accessibility_granted: true,
                apps: Vec::new(),
                elements: Vec::new(),
                top_level_by_process: HashMap::new(),
                menu_bar_by_process: HashMap::new(),
                recorded_actions: Vec::new(),
                captured_regions: Vec::new(),
                captured_windows: Vec::new(),
                capture_options: Vec::new(),
                script_result: Ok(String::new()),
                not_installed_apps: Vec::new(),
                element_read_count: 0,
                apps_ignoring_quit: Vec::new(),
                clipboard_text: None,
            }),
        }
    }

    pub fn add_app(&self, name: &str, bundle_id: Option<&str>) -> ProcessId {
        let mut state = self.locked();
        let offset = i32::try_from(state.apps.len()).unwrap_or(i32::MAX - FIRST_FAKE_PROCESS_ID);
        let process_id = ProcessId(FIRST_FAKE_PROCESS_ID + offset);
        state.apps.push(AppInfo {
            name: name.to_owned(),
            bundle_id: bundle_id.map(str::to_owned),
            process_id,
            is_active: false,
        });
        process_id
    }

    pub fn make_frontmost(&self, process_id: ProcessId) {
        for app in &mut self.locked().apps {
            app.is_active = app.process_id == process_id;
        }
    }

    pub fn add_window(
        &self,
        process_id: ProcessId,
        attributes: ElementAttributes,
    ) -> FakeElementId {
        let mut state = self.locked();
        let window = Self::push_element(&mut state, attributes);
        state
            .top_level_by_process
            .entry(process_id)
            .or_default()
            .push(window);
        window
    }

    pub fn add_child(&self, parent: FakeElementId, attributes: ElementAttributes) -> FakeElementId {
        let mut state = self.locked();
        let child = Self::push_element(&mut state, attributes);
        state.elements[parent.0].children.push(child);
        child
    }

    pub fn set_menu_bar(
        &self,
        process_id: ProcessId,
        attributes: ElementAttributes,
    ) -> FakeElementId {
        let mut state = self.locked();
        let menu_bar = Self::push_element(&mut state, attributes);
        state.menu_bar_by_process.insert(process_id, menu_bar);
        menu_bar
    }

    pub fn set_value(&self, element: FakeElementId, value: &str) {
        self.locked().elements[element.0].attributes.value = Some(value.to_owned());
    }

    pub fn make_unreadable(&self, element: FakeElementId) {
        self.locked().elements[element.0].is_unreadable = true;
    }

    /// The element has no context menu action.
    pub fn make_show_menu_fail(&self, element: FakeElementId) {
        self.locked().elements[element.0].does_show_menu_fail = true;
    }

    /// Asking the element for focus does nothing; pressing it still focuses it.
    pub fn make_focus_fail(&self, element: FakeElementId) {
        self.locked().elements[element.0].does_focus_fail = true;
    }

    pub fn make_press_fail(&self, element: FakeElementId) {
        self.locked().elements[element.0].does_press_fail = true;
    }

    /// The element reports success for value writes but keeps its old value.
    pub fn make_set_value_ignored(&self, element: FakeElementId) {
        self.locked().elements[element.0].is_set_value_ignored = true;
    }

    /// Opening an app with this name fails as if it weren't installed.
    pub fn make_not_installed(&self, name: &str) {
        self.locked().not_installed_apps.push(name.to_owned());
    }

    /// The element has no scroll-to-visible action, like most controls outside scroll areas.
    pub fn make_scroll_unsupported(&self, element: FakeElementId) {
        self.locked().elements[element.0].is_scroll_unsupported = true;
    }

    /// The window accepts minimize requests but stays as it is, as under Stage Manager.
    pub fn make_minimize_ignored(&self, window: FakeElementId) {
        self.locked().elements[window.0].is_minimize_ignored = true;
    }

    /// Makes the app ignore polite quit requests, like an app showing a save prompt.
    pub fn make_ignore_quit(&self, process_id: ProcessId) {
        self.locked().apps_ignoring_quit.push(process_id);
    }

    /// Another app's window sits on top of the element, so hit tests at it fail.
    pub fn make_covered(&self, element: FakeElementId) {
        self.locked().elements[element.0].is_covered_by_another_app = true;
    }

    pub fn deny_accessibility_permission(&self) {
        self.locked().is_accessibility_granted = false;
    }

    pub fn set_script_result(&self, result: DeskwrightResult<String>) {
        self.locked().script_result = result;
    }

    /// Replaces the registry's contents with just this element and returns its ref.
    pub fn register(&self, registry: &ElementRegistry, element: FakeElementId) -> ElementRef {
        self.register_all(registry, &[element])[0]
    }

    /// Like [`Self::register`], for several elements in one snapshot.
    pub fn register_all(
        &self,
        registry: &ElementRegistry,
        elements: &[FakeElementId],
    ) -> Vec<ElementRef> {
        let entries: Vec<(ElementRef, NativeElement)> = elements
            .iter()
            .map(|&element| {
                (
                    ElementRef::from_index(element.0 + 1),
                    NativeElement::new(element),
                )
            })
            .collect();
        let element_refs = entries
            .iter()
            .map(|(element_ref, _)| *element_ref)
            .collect();
        registry.replace_all(entries.into_iter().collect::<HashMap<_, _>>());
        element_refs
    }

    /// How many times an element's attributes were read, to check how much of a tree a walk
    /// visited.
    pub fn element_read_count(&self) -> usize {
        self.locked().element_read_count
    }

    pub fn recorded_actions(&self) -> Vec<RecordedAction> {
        self.locked().recorded_actions.clone()
    }

    pub fn captured_regions(&self) -> Vec<Bounds> {
        self.locked().captured_regions.clone()
    }

    /// The process and bounds of every window captured, in call order.
    pub fn captured_windows(&self) -> Vec<(ProcessId, Bounds)> {
        self.locked().captured_windows.clone()
    }

    /// The options of every capture, in call order.
    pub fn capture_options(&self) -> Vec<CaptureOptions> {
        self.locked().capture_options.clone()
    }

    /// Minimizes or restores a window directly, without recording an action.
    pub fn set_minimized(&self, window: FakeElementId, is_minimized: bool) {
        self.locked().elements[window.0].is_minimized = is_minimized;
    }

    fn push_element(state: &mut DesktopState, attributes: ElementAttributes) -> FakeElementId {
        state.elements.push(FakeElement {
            attributes,
            ..FakeElement::default()
        });
        FakeElementId(state.elements.len() - 1)
    }

    fn element_id(element: &NativeElement) -> DeskwrightResult<FakeElementId> {
        element
            .downcast_ref::<FakeElementId>()
            .copied()
            .ok_or_else(|| {
                DeskwrightError::operation_failed(
                    "resolve element",
                    "the handle was not made by FakeDesktop",
                )
            })
    }

    /// Pressing a checkbox flips its value between "0" and "1", as on a real desktop.
    fn toggle_if_checkbox(&self, id: FakeElementId) {
        let mut state = self.locked();
        let attributes = &mut state.elements[id.0].attributes;
        if attributes.role == CHECKBOX_ROLE {
            let is_checked = attributes.value.as_deref() == Some(CHECKED_VALUE);
            let flipped = if is_checked {
                UNCHECKED_VALUE
            } else {
                CHECKED_VALUE
            };
            attributes.value = Some(flipped.to_owned());
        }
    }

    /// Pressing or focusing an element moves keyboard focus to it, as on a real desktop.
    fn move_focus_to(&self, id: FakeElementId) {
        let mut state = self.locked();
        for (index, fake_element) in state.elements.iter_mut().enumerate() {
            fake_element.attributes.is_focused = index == id.0;
        }
    }

    fn record(&self, action: RecordedAction) {
        self.locked().recorded_actions.push(action);
    }

    fn locked(&self) -> MutexGuard<'_, DesktopState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl AccessibilityPermission for FakeDesktop {
    fn is_granted(&self) -> bool {
        self.locked().is_accessibility_granted
    }
}

impl AppDirectory for FakeDesktop {
    fn running_apps(&self) -> DeskwrightResult<Vec<AppInfo>> {
        Ok(self.locked().apps.clone())
    }

    fn frontmost_app(&self) -> DeskwrightResult<Option<AppInfo>> {
        Ok(self.locked().apps.iter().find(|app| app.is_active).cloned())
    }
}

impl ElementTreeReader for FakeDesktop {
    fn top_level_elements(&self, process_id: ProcessId) -> DeskwrightResult<Vec<NativeElement>> {
        let state = self.locked();
        let windows = state.top_level_by_process.get(&process_id);
        Ok(windows
            .into_iter()
            .flatten()
            .map(|&window| NativeElement::new(window))
            .collect())
    }

    fn attributes(&self, element: &NativeElement) -> DeskwrightResult<ElementAttributes> {
        let id = Self::element_id(element)?;
        let mut state = self.locked();
        state.element_read_count += 1;
        let fake_element = &state.elements[id.0];
        if fake_element.is_unreadable {
            return Err(DeskwrightError::operation_failed(
                "read attributes",
                "element is gone",
            ));
        }
        Ok(fake_element.attributes.clone())
    }

    fn children(&self, element: &NativeElement) -> DeskwrightResult<Vec<NativeElement>> {
        let id = Self::element_id(element)?;
        let state = self.locked();
        Ok(state.elements[id.0]
            .children
            .iter()
            .map(|&child| NativeElement::new(child))
            .collect())
    }
}

impl MenuBarLocator for FakeDesktop {
    fn menu_bar(&self, process_id: ProcessId) -> DeskwrightResult<NativeElement> {
        self.locked()
            .menu_bar_by_process
            .get(&process_id)
            .map(|&menu_bar| NativeElement::new(menu_bar))
            .ok_or_else(|| {
                DeskwrightError::operation_failed("find menu bar", "app has no menu bar")
            })
    }
}

impl ElementActions for FakeDesktop {
    fn press(&self, element: &NativeElement) -> DeskwrightResult<()> {
        let id = Self::element_id(element)?;
        if self.locked().elements[id.0].does_press_fail {
            return Err(DeskwrightError::operation_failed(
                "press",
                "element has no press action",
            ));
        }
        self.move_focus_to(id);
        self.toggle_if_checkbox(id);
        self.record(RecordedAction::Press(id));
        Ok(())
    }

    fn focus(&self, element: &NativeElement) -> DeskwrightResult<()> {
        let id = Self::element_id(element)?;
        if !self.locked().elements[id.0].does_focus_fail {
            self.move_focus_to(id);
        }
        self.record(RecordedAction::Focus(id));
        Ok(())
    }

    fn set_value(&self, element: &NativeElement, value: &str) -> DeskwrightResult<()> {
        let id = Self::element_id(element)?;
        {
            let mut state = self.locked();
            let fake_element = &mut state.elements[id.0];
            if !fake_element.is_set_value_ignored {
                fake_element.attributes.value = Some(value.to_owned());
            }
        }
        self.record(RecordedAction::SetValue(id, value.to_owned()));
        Ok(())
    }

    fn show_menu(&self, element: &NativeElement) -> DeskwrightResult<()> {
        let id = Self::element_id(element)?;
        if self.locked().elements[id.0].does_show_menu_fail {
            return Err(DeskwrightError::operation_failed(
                "show the menu",
                "element has no menu",
            ));
        }
        self.record(RecordedAction::ShowMenu(id));
        Ok(())
    }

    fn cancel(&self, element: &NativeElement) -> DeskwrightResult<()> {
        let id = Self::element_id(element)?;
        self.record(RecordedAction::Cancel(id));
        Ok(())
    }

    fn scroll_into_view(&self, element: &NativeElement) -> DeskwrightResult<()> {
        let id = Self::element_id(element)?;
        if self.locked().elements[id.0].is_scroll_unsupported {
            return Err(DeskwrightError::operation_failed(
                "scroll the element into view",
                "the element does not support this action",
            ));
        }
        self.record(RecordedAction::ScrollIntoView(id));
        Ok(())
    }
}

impl WindowController for FakeDesktop {
    fn is_minimized(&self, window: &NativeElement) -> DeskwrightResult<bool> {
        let id = Self::element_id(window)?;
        Ok(self.locked().elements[id.0].is_minimized)
    }

    fn set_minimized(&self, window: &NativeElement, should_minimize: bool) -> DeskwrightResult<()> {
        let id = Self::element_id(window)?;
        {
            let mut state = self.locked();
            let fake_window = &mut state.elements[id.0];
            if !fake_window.is_minimize_ignored {
                fake_window.is_minimized = should_minimize;
            }
        }
        self.record(RecordedAction::SetMinimized(id, should_minimize));
        Ok(())
    }

    fn move_to(&self, window: &NativeElement, origin: Point) -> DeskwrightResult<()> {
        let id = Self::element_id(window)?;
        let mut state = self.locked();
        let bounds = state.elements[id.0]
            .attributes
            .bounds
            .get_or_insert(Bounds {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            });
        bounds.x = origin.x;
        bounds.y = origin.y;
        state
            .recorded_actions
            .push(RecordedAction::MoveWindow(id, origin));
        Ok(())
    }

    fn resize(&self, window: &NativeElement, size: Size) -> DeskwrightResult<()> {
        let id = Self::element_id(window)?;
        let mut state = self.locked();
        let bounds = state.elements[id.0]
            .attributes
            .bounds
            .get_or_insert(Bounds {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            });
        bounds.width = size.width;
        bounds.height = size.height;
        state
            .recorded_actions
            .push(RecordedAction::ResizeWindow(id, size));
        Ok(())
    }

    /// Closing removes the window from its app's windows, as on a real desktop.
    fn close(&self, window: &NativeElement) -> DeskwrightResult<()> {
        let id = Self::element_id(window)?;
        let mut state = self.locked();
        for windows in state.top_level_by_process.values_mut() {
            windows.retain(|&window| window != id);
        }
        state.recorded_actions.push(RecordedAction::CloseWindow(id));
        Ok(())
    }

    fn raise(&self, window: &NativeElement) -> DeskwrightResult<()> {
        let id = Self::element_id(window)?;
        self.record(RecordedAction::Raise(id));
        Ok(())
    }
}

impl AppTerminator for FakeDesktop {
    /// Quitting removes the app, unless it was set to ignore polite quit requests (like an app
    /// showing a save prompt) and `force` is false.
    fn quit(&self, process_id: ProcessId, force: bool) -> DeskwrightResult<()> {
        let mut state = self.locked();
        state
            .recorded_actions
            .push(RecordedAction::QuitApp { process_id, force });
        let ignores_request = !force && state.apps_ignoring_quit.contains(&process_id);
        if !ignores_request {
            state.apps.retain(|app| app.process_id != process_id);
        }
        Ok(())
    }
}

impl Clipboard for FakeDesktop {
    fn read_text(&self) -> DeskwrightResult<Option<String>> {
        Ok(self.locked().clipboard_text.clone())
    }

    fn write_text(&self, text: &str) -> DeskwrightResult<()> {
        self.locked().clipboard_text = Some(text.to_owned());
        Ok(())
    }
}

impl AppLauncher for FakeDesktop {
    /// Activates a matching running app, or "launches" a new one, like the real launcher.
    fn open(&self, name_or_bundle_id: &str, address: Option<&WebAddress>) -> DeskwrightResult<()> {
        let is_not_installed = self
            .locked()
            .not_installed_apps
            .iter()
            .any(|name| name.eq_ignore_ascii_case(name_or_bundle_id));
        if is_not_installed {
            return Err(DeskwrightError::AppNotInstalled(
                name_or_bundle_id.to_owned(),
            ));
        }
        let running = self
            .running_apps()?
            .into_iter()
            .find(|app| app.is_identified_by(name_or_bundle_id));
        let process_id = match running {
            Some(app) => app.process_id,
            None => self.add_app(name_or_bundle_id, None),
        };
        self.make_frontmost(process_id);
        self.record(RecordedAction::OpenApp {
            name_or_bundle_id: name_or_bundle_id.to_owned(),
            address: address.map(|address| address.as_str().to_owned()),
        });
        Ok(())
    }
}

impl HitTester for FakeDesktop {
    fn is_app_of_element_on_top_at(
        &self,
        element: &NativeElement,
        _point: Point,
    ) -> DeskwrightResult<bool> {
        let id = Self::element_id(element)?;
        Ok(!self.locked().elements[id.0].is_covered_by_another_app)
    }
}

impl InputSynthesizer for FakeDesktop {
    fn type_text(&self, text: &str) -> DeskwrightResult<()> {
        self.record(RecordedAction::TypeText(text.to_owned()));
        Ok(())
    }

    fn click_at(&self, point: Point) -> DeskwrightResult<()> {
        self.record(RecordedAction::ClickAt(point));
        Ok(())
    }

    fn double_click_at(&self, point: Point) -> DeskwrightResult<()> {
        self.record(RecordedAction::DoubleClickAt(point));
        Ok(())
    }

    fn right_click_at(&self, point: Point) -> DeskwrightResult<()> {
        self.record(RecordedAction::RightClickAt(point));
        Ok(())
    }

    fn move_pointer_to(&self, point: Point) -> DeskwrightResult<()> {
        self.record(RecordedAction::MovePointerTo(point));
        Ok(())
    }

    fn drag(&self, from: Point, to: Point) -> DeskwrightResult<()> {
        self.record(RecordedAction::Drag { from, to });
        Ok(())
    }

    fn press_chord(&self, chord: &KeyChord) -> DeskwrightResult<()> {
        self.record(RecordedAction::PressChord(chord.clone()));
        Ok(())
    }
}

impl FakeDesktop {
    /// Records the options and returns placeholder bytes in the requested format.
    fn fake_image(&self, options: &CaptureOptions) -> EncodedImage {
        self.locked().capture_options.push(*options);
        EncodedImage::new(options.format, FAKE_PNG_SIGNATURE.to_vec())
    }
}

impl ScreenCapturer for FakeDesktop {
    fn capture_main_display(&self, options: &CaptureOptions) -> DeskwrightResult<EncodedImage> {
        Ok(self.fake_image(options))
    }

    fn capture_region(
        &self,
        region: Bounds,
        options: &CaptureOptions,
    ) -> DeskwrightResult<EncodedImage> {
        self.locked().captured_regions.push(region);
        Ok(self.fake_image(options))
    }

    fn capture_window(
        &self,
        process_id: ProcessId,
        bounds: Bounds,
        options: &CaptureOptions,
    ) -> DeskwrightResult<EncodedImage> {
        self.locked().captured_windows.push((process_id, bounds));
        Ok(self.fake_image(options))
    }
}

impl ScriptRunner for FakeDesktop {
    fn run(&self, script: &Script) -> DeskwrightResult<String> {
        self.record(RecordedAction::RunScript(script.source().to_owned()));
        self.locked().script_result.clone()
    }
}
