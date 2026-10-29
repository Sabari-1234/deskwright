use std::sync::Arc;

use crate::error::DeskwrightResult;
use crate::model::native_element::NativeElement;
use crate::ports::ElementTreeReader;

const MENU_ROLE: &str = "AXMenu";

/// Finds items in menus by title. Items sit directly under their parent, or one level down
/// inside an untitled menu container (menu bar item → menu → menu items; pop-up button →
/// menu → items), so both levels are searched.
#[derive(Clone)]
pub struct MenuNavigator {
    tree: Arc<dyn ElementTreeReader>,
}

impl MenuNavigator {
    pub fn new(tree: Arc<dyn ElementTreeReader>) -> Self {
        Self { tree }
    }

    /// The item titled `title` (ignoring ASCII case) under `parent`, if any.
    pub fn find_item(
        &self,
        parent: &NativeElement,
        title: &str,
    ) -> DeskwrightResult<Option<NativeElement>> {
        let found = self
            .titled_items(parent)?
            .into_iter()
            .find(|(item_title, _)| item_title.eq_ignore_ascii_case(title))
            .map(|(_, item)| item);
        Ok(found)
    }

    /// The titles of every item under `parent`, for error messages.
    pub fn item_titles(&self, parent: &NativeElement) -> DeskwrightResult<Vec<String>> {
        Ok(self
            .titled_items(parent)?
            .into_iter()
            .map(|(title, _)| title)
            .collect())
    }

    /// The menus directly under `parent`, such as a pop-up button's open menu.
    pub fn menus_of(&self, parent: &NativeElement) -> DeskwrightResult<Vec<NativeElement>> {
        let mut menus = Vec::new();
        for child in self.tree.children(parent)? {
            if self.tree.attributes(&child)?.role == MENU_ROLE {
                menus.push(child);
            }
        }
        Ok(menus)
    }

    fn titled_items(
        &self,
        parent: &NativeElement,
    ) -> DeskwrightResult<Vec<(String, NativeElement)>> {
        let mut items = Vec::new();
        for child in self.tree.children(parent)? {
            match self.title_of(&child)? {
                Some(title) => items.push((title, child)),
                None => {
                    for grandchild in self.tree.children(&child)? {
                        if let Some(title) = self.title_of(&grandchild)? {
                            items.push((title, grandchild));
                        }
                    }
                }
            }
        }
        Ok(items)
    }

    fn title_of(&self, element: &NativeElement) -> DeskwrightResult<Option<String>> {
        Ok(self
            .tree
            .attributes(element)?
            .title
            .filter(|title| !title.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDesktop, element};

    #[test]
    fn finds_items_directly_and_inside_untitled_containers() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("TextEdit", None);
        let menu_bar = desktop.set_menu_bar(process_id, element("AXMenuBar", None));
        let file = desktop.add_child(menu_bar, element("AXMenuBarItem", Some("File")));
        let file_menu = desktop.add_child(file, element("AXMenu", None));
        desktop.add_child(file_menu, element("AXMenuItem", Some("Save")));
        desktop.add_child(file_menu, element("AXMenuItem", Some("")));
        let navigator = MenuNavigator::new(desktop.clone());
        let found_title = |parent, title| {
            navigator
                .find_item(&NativeElement::new(parent), title)
                .unwrap()
                .map(|item| desktop.attributes(&item).unwrap().title.unwrap())
        };

        assert_eq!(found_title(menu_bar, "FILE"), Some("File".to_owned()));
        assert_eq!(found_title(file, "save"), Some("Save".to_owned()));
        assert_eq!(found_title(file, "Print"), None);
        assert_eq!(
            navigator.item_titles(&NativeElement::new(file)),
            Ok(vec!["Save".to_owned()])
        );
        assert_eq!(
            navigator.menus_of(&NativeElement::new(file)).unwrap().len(),
            1
        );
        assert!(
            navigator
                .menus_of(&NativeElement::new(menu_bar))
                .unwrap()
                .is_empty()
        );
    }
}
