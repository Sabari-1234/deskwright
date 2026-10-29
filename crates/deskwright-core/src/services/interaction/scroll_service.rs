use std::sync::Arc;

use crate::element_registry::ElementRegistry;
use crate::error::DeskwrightResult;
use crate::model::element::ElementRef;
use crate::ports::{ElementActions, ElementTreeReader};

/// What [`ScrollService::scroll_into_view`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollOutcome {
    Scrolled,
    /// The app can't scroll the element, but it is already on screen.
    AlreadyOnScreen,
}

/// Scrolls an element's containers until it is visible, like Playwright's
/// `scrollIntoViewIfNeeded()`.
pub struct ScrollService {
    registry: Arc<ElementRegistry>,
    actions: Arc<dyn ElementActions>,
    tree: Arc<dyn ElementTreeReader>,
}

impl ScrollService {
    pub fn new(
        registry: Arc<ElementRegistry>,
        actions: Arc<dyn ElementActions>,
        tree: Arc<dyn ElementTreeReader>,
    ) -> Self {
        Self {
            registry,
            actions,
            tree,
        }
    }

    /// Many controls outside scroll areas don't support scrolling at all; for those, being
    /// on screen already is success, as "if needed" implies.
    pub fn scroll_into_view(&self, element_ref: &ElementRef) -> DeskwrightResult<ScrollOutcome> {
        let element = self.registry.resolve(element_ref)?;
        match self.actions.scroll_into_view(&element) {
            Ok(()) => Ok(ScrollOutcome::Scrolled),
            Err(error) => match self.tree.on_screen_bounds(&element) {
                Ok(Some(_)) => Ok(ScrollOutcome::AlreadyOnScreen),
                _ => Err(error),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::geometry::Bounds;
    use crate::testing::{FakeDesktop, FakeElementId, RecordedAction, element};

    #[test]
    fn asks_the_element_to_scroll_into_view() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Mail", None);
        let row = desktop.add_window(process_id, element("AXRow", Some("Invoice")));
        let registry = Arc::new(ElementRegistry::default());
        let row_ref = desktop.register(&registry, row);

        let outcome = ScrollService::new(registry, desktop.clone(), desktop.clone())
            .scroll_into_view(&row_ref)
            .unwrap();

        assert_eq!(outcome, ScrollOutcome::Scrolled);
        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::ScrollIntoView(row)]
        );
    }

    fn unscrollable(desktop: &FakeDesktop, bounds: Option<Bounds>) -> FakeElementId {
        let process_id = desktop.add_app("Finder", None);
        let mut attributes = element("AXCheckBox", Some("Show item info"));
        attributes.bounds = bounds;
        let checkbox = desktop.add_window(process_id, attributes);
        desktop.make_scroll_unsupported(checkbox);
        checkbox
    }

    #[test]
    fn accepts_an_unscrollable_element_that_is_already_on_screen() {
        let desktop = Arc::new(FakeDesktop::new());
        let checkbox = unscrollable(
            &desktop,
            Some(Bounds {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            }),
        );
        let registry = Arc::new(ElementRegistry::default());
        let checkbox_ref = desktop.register(&registry, checkbox);

        let outcome = ScrollService::new(registry, desktop.clone(), desktop.clone())
            .scroll_into_view(&checkbox_ref)
            .unwrap();

        assert_eq!(outcome, ScrollOutcome::AlreadyOnScreen);
    }

    #[test]
    fn fails_for_an_unscrollable_element_that_is_off_screen() {
        let desktop = Arc::new(FakeDesktop::new());
        let checkbox = unscrollable(&desktop, None);
        let registry = Arc::new(ElementRegistry::default());
        let checkbox_ref = desktop.register(&registry, checkbox);

        let result = ScrollService::new(registry, desktop.clone(), desktop.clone())
            .scroll_into_view(&checkbox_ref);

        assert!(result.is_err());
    }
}
