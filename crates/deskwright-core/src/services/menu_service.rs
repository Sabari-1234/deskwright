use std::sync::Arc;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::app::AppSelector;
use crate::model::menu_path::MenuPath;
use crate::ports::{AccessibilityPermission, ElementActions, ElementTreeReader, MenuBarLocator};
use crate::services::app_resolver::AppResolver;
use crate::services::menu_navigator::MenuNavigator;

/// Chooses a menu-bar item by its path, like `File > Save As…`.
pub struct MenuService {
    apps: AppResolver,
    permission: Arc<dyn AccessibilityPermission>,
    menu_bars: Arc<dyn MenuBarLocator>,
    navigator: MenuNavigator,
    actions: Arc<dyn ElementActions>,
}

impl MenuService {
    pub fn new(
        apps: AppResolver,
        permission: Arc<dyn AccessibilityPermission>,
        menu_bars: Arc<dyn MenuBarLocator>,
        tree: Arc<dyn ElementTreeReader>,
        actions: Arc<dyn ElementActions>,
    ) -> Self {
        Self {
            apps,
            permission,
            menu_bars,
            navigator: MenuNavigator::new(tree),
            actions,
        }
    }

    /// Walks the menu tree title by title, then presses the final item.
    pub fn choose(&self, selector: &AppSelector, path: &MenuPath) -> DeskwrightResult<()> {
        self.permission.ensure_granted()?;
        let app = self.apps.resolve(selector)?;
        let mut current = self.menu_bars.menu_bar(app.process_id)?;
        for item_title in path.items() {
            current = self
                .navigator
                .find_item(&current, item_title)?
                .ok_or_else(|| DeskwrightError::MenuItemNotFound {
                    path: path.to_string(),
                    missing_item: item_title.clone(),
                })?;
        }
        self.actions.press(&current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDesktop, RecordedAction, element};

    fn menu_service(desktop: &Arc<FakeDesktop>) -> MenuService {
        MenuService::new(
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            desktop.clone(),
            desktop.clone(),
        )
    }

    #[test]
    fn presses_the_final_item_through_untitled_menu_containers() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("TextEdit", None);
        desktop.make_frontmost(process_id);
        let menu_bar = desktop.set_menu_bar(process_id, element("AXMenuBar", None));
        let file = desktop.add_child(menu_bar, element("AXMenuBarItem", Some("File")));
        let file_menu = desktop.add_child(file, element("AXMenu", None));
        desktop.add_child(file_menu, element("AXMenuItem", Some("Open…")));
        let save_as = desktop.add_child(file_menu, element("AXMenuItem", Some("Save As…")));
        let path = MenuPath::parse("file > save as…").unwrap();

        menu_service(&desktop)
            .choose(&AppSelector::Frontmost, &path)
            .unwrap();

        assert_eq!(desktop.recorded_actions(), [RecordedAction::Press(save_as)]);
    }

    #[test]
    fn names_the_missing_item_when_the_path_does_not_exist() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("TextEdit", None);
        desktop.make_frontmost(process_id);
        let menu_bar = desktop.set_menu_bar(process_id, element("AXMenuBar", None));
        desktop.add_child(menu_bar, element("AXMenuBarItem", Some("File")));
        let path = MenuPath::parse("File > Print").unwrap();

        let error = menu_service(&desktop)
            .choose(&AppSelector::Frontmost, &path)
            .unwrap_err();

        assert_eq!(
            error,
            DeskwrightError::MenuItemNotFound {
                path: "File > Print".to_owned(),
                missing_item: "Print".to_owned(),
            }
        );
        assert!(desktop.recorded_actions().is_empty());
    }
}
