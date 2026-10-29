use std::sync::Arc;
use std::time::Duration;

use crate::element_registry::ElementRegistry;
use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::app::{AppInfo, AppSelector};
use crate::model::element::ElementRef;
use crate::model::native_element::NativeElement;
use crate::polling::wait_until;
use crate::ports::{ElementActions, ElementTreeReader};
use crate::services::app_resolver::AppResolver;
use crate::services::menu_navigator::MenuNavigator;

const OPTION_POLL_INTERVAL: Duration = Duration::from_millis(20);
const MENU_ROLE: &str = "AXMenu";

/// Chooses an option from a pop-up button or menu button, like Playwright's `selectOption()`:
/// opens the pop-up, waits for its menu, and presses the item with the given title.
pub struct OptionService {
    registry: Arc<ElementRegistry>,
    apps: AppResolver,
    tree: Arc<dyn ElementTreeReader>,
    actions: Arc<dyn ElementActions>,
    navigator: MenuNavigator,
    menu_timeout: Duration,
}

impl OptionService {
    /// `menu_timeout` is how long to wait for the opened menu to list the option.
    pub fn new(
        registry: Arc<ElementRegistry>,
        apps: AppResolver,
        tree: Arc<dyn ElementTreeReader>,
        actions: Arc<dyn ElementActions>,
        menu_timeout: Duration,
    ) -> Self {
        Self {
            registry,
            apps,
            navigator: MenuNavigator::new(tree.clone()),
            tree,
            actions,
            menu_timeout,
        }
    }

    /// `app` is the app the pop-up belongs to; its open menus are searched as well.
    pub fn select(
        &self,
        element_ref: &ElementRef,
        app: &AppSelector,
        option: &str,
    ) -> DeskwrightResult<()> {
        let pop_up = self.registry.resolve(element_ref)?;
        let app = self.apps.resolve(app)?;
        self.actions.press(&pop_up)?;
        match self.wait_for_option(&pop_up, &app, option) {
            Some(item) => self.actions.press(&item),
            None => {
                let menus = self.open_menus(&pop_up, &app);
                let error = self.option_not_found(&menus, option);
                self.close(&menus);
                Err(error)
            }
        }
    }

    fn wait_for_option(
        &self,
        pop_up: &NativeElement,
        app: &AppInfo,
        option: &str,
    ) -> Option<NativeElement> {
        let mut found = None;
        wait_until(self.menu_timeout, OPTION_POLL_INTERVAL, || {
            found = self
                .open_menus(pop_up, app)
                .iter()
                .find_map(|menu| self.navigator.find_item(menu, option).ok().flatten());
            found.is_some()
        });
        found
    }

    /// The pop-up's menu: AppKit exposes it under the pop-up the first time it opens, and
    /// afterwards under the app itself, so both places are searched.
    fn open_menus(&self, pop_up: &NativeElement, app: &AppInfo) -> Vec<NativeElement> {
        let mut menus = self.navigator.menus_of(pop_up).unwrap_or_default();
        for top_level in self
            .tree
            .top_level_elements(app.process_id)
            .unwrap_or_default()
        {
            let is_menu = self
                .tree
                .attributes(&top_level)
                .is_ok_and(|attributes| attributes.role == MENU_ROLE);
            if is_menu {
                menus.push(top_level);
            }
        }
        menus
    }

    /// Cancels the open menus. Left open, a menu keeps AppKit tracking it and swallows later
    /// actions. A failure to close is less useful to report than why the option was missing,
    /// so it is ignored.
    fn close(&self, menus: &[NativeElement]) {
        for menu in menus {
            let _ = self.actions.cancel(menu);
        }
    }

    fn option_not_found(&self, menus: &[NativeElement], option: &str) -> DeskwrightError {
        let available: Vec<String> = menus
            .iter()
            .flat_map(|menu| self.navigator.item_titles(menu).unwrap_or_default())
            .collect();
        let reason = if available.is_empty() {
            format!("no option \"{option}\"; the element listed no options")
        } else {
            format!(
                "no option \"{option}\"; available: {}",
                available.join(", ")
            )
        };
        DeskwrightError::operation_failed("select the option", reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDesktop, FakeElementId, RecordedAction, element};

    fn pop_up_with_options(desktop: &FakeDesktop) -> (FakeElementId, FakeElementId) {
        let (pop_up, png, _) = pop_up_with_menu(desktop);
        (pop_up, png)
    }

    fn pop_up_with_menu(desktop: &FakeDesktop) -> (FakeElementId, FakeElementId, FakeElementId) {
        let process_id = desktop.add_app("Preview", None);
        let window = desktop.add_window(process_id, element("AXWindow", Some("Export")));
        let pop_up = desktop.add_child(window, element("AXPopUpButton", Some("Format")));
        let menu = desktop.add_child(pop_up, element("AXMenu", None));
        desktop.add_child(menu, element("AXMenuItem", Some("JPEG")));
        let png = desktop.add_child(menu, element("AXMenuItem", Some("PNG")));
        (pop_up, png, menu)
    }

    fn option_service(desktop: &Arc<FakeDesktop>, registry: Arc<ElementRegistry>) -> OptionService {
        OptionService::new(
            registry,
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            Duration::ZERO,
        )
    }

    fn preview() -> AppSelector {
        AppSelector::NameOrBundleId("Preview".to_owned())
    }

    #[test]
    fn opens_the_pop_up_and_presses_the_option() {
        let desktop = Arc::new(FakeDesktop::new());
        let (pop_up, png) = pop_up_with_options(&desktop);
        let registry = Arc::new(ElementRegistry::default());
        let pop_up_ref = desktop.register(&registry, pop_up);

        option_service(&desktop, registry)
            .select(&pop_up_ref, &preview(), "png")
            .unwrap();

        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::Press(pop_up), RecordedAction::Press(png)]
        );
    }

    #[test]
    fn lists_the_available_options_when_the_option_is_missing() {
        let desktop = Arc::new(FakeDesktop::new());
        let (pop_up, _, menu) = pop_up_with_menu(&desktop);
        let registry = Arc::new(ElementRegistry::default());
        let pop_up_ref = desktop.register(&registry, pop_up);

        let error = option_service(&desktop, registry)
            .select(&pop_up_ref, &preview(), "TIFF")
            .unwrap_err();

        assert!(error.to_string().contains("available: JPEG, PNG"));
        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::Press(pop_up), RecordedAction::Cancel(menu)]
        );
    }

    #[test]
    fn says_when_the_element_has_no_options() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Preview", None);
        let button = desktop.add_window(process_id, element("AXButton", Some("OK")));
        let registry = Arc::new(ElementRegistry::default());
        let button_ref = desktop.register(&registry, button);

        let error = option_service(&desktop, registry)
            .select(&button_ref, &preview(), "PNG")
            .unwrap_err();

        assert!(error.to_string().contains("listed no options"));
    }

    #[test]
    fn finds_the_menu_when_the_app_shows_it_at_the_top_level() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Preview", None);
        let window = desktop.add_window(process_id, element("AXWindow", Some("Export")));
        let pop_up = desktop.add_child(window, element("AXPopUpButton", Some("Format")));
        let open_menu = desktop.add_window(process_id, element("AXMenu", None));
        let heic = desktop.add_child(open_menu, element("AXMenuItem", Some("HEIC")));
        let registry = Arc::new(ElementRegistry::default());
        let pop_up_ref = desktop.register(&registry, pop_up);
        let service = option_service(&desktop, registry);

        service.select(&pop_up_ref, &preview(), "heic").unwrap();
        let error = service.select(&pop_up_ref, &preview(), "TIFF").unwrap_err();

        assert!(
            desktop
                .recorded_actions()
                .contains(&RecordedAction::Press(heic))
        );
        assert!(error.to_string().contains("available: HEIC"));
        assert_eq!(
            desktop.recorded_actions().last(),
            Some(&RecordedAction::Cancel(open_menu))
        );
    }
}
