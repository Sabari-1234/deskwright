use std::sync::Arc;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::element::ElementRef;
use crate::model::element_target::{ElementTarget, Locator, MatchPosition};
use crate::services::element_wait_service::ElementWaitService;
use crate::services::snapshot_service::MAX_SNAPSHOT_ELEMENTS;

/// Turns an [`ElementTarget`] into a usable ref. Locators auto-wait, so an action can name an
/// element that is still appearing (a dialog opening, a page loading) in a single call.
pub struct ElementLocatorService {
    waits: Arc<ElementWaitService>,
}

impl ElementLocatorService {
    pub fn new(waits: Arc<ElementWaitService>) -> Self {
        Self { waits }
    }

    pub fn resolve(&self, target: &ElementTarget) -> DeskwrightResult<ElementRef> {
        match target {
            ElementTarget::Ref(element_ref) => Ok(*element_ref),
            ElementTarget::Locator(locator) => self.locate(locator),
        }
    }

    fn locate(&self, locator: &Locator) -> DeskwrightResult<ElementRef> {
        let (minimum_matches, limit) = match locator.position {
            MatchPosition::Nth(nth) => (nth + 1, nth + 1),
            // The last match is only known once the whole tree has been searched.
            MatchPosition::Last => (1, MAX_SNAPSHOT_ELEMENTS),
        };
        let results = self.waits.wait_for_matches(
            &locator.app,
            &locator.query,
            minimum_matches,
            limit,
            locator.timeout,
        )?;
        let picked = match locator.position {
            MatchPosition::Nth(nth) => results.matches.get(nth),
            MatchPosition::Last => results.matches.last(),
        };
        picked.map(|summary| summary.element_ref).ok_or_else(|| {
            DeskwrightError::InvalidInput(format!(
                "only {} elements match {}",
                results.matches.len(),
                locator.query
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::element_registry::ElementRegistry;
    use crate::model::app::AppSelector;
    use crate::model::element_query::ElementQuery;
    use crate::services::app_resolver::AppResolver;
    use crate::services::element_search_service::ElementSearchService;
    use crate::services::snapshot_service::SnapshotService;
    use crate::testing::{FakeDesktop, element};

    fn locator_service(
        desktop: &Arc<FakeDesktop>,
    ) -> (ElementLocatorService, Arc<ElementRegistry>) {
        let registry = Arc::new(ElementRegistry::default());
        let snapshots = Arc::new(SnapshotService::new(
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            registry.clone(),
        ));
        let waits = Arc::new(ElementWaitService::new(
            Arc::new(ElementSearchService::new(snapshots)),
            Duration::ZERO,
        ));
        (ElementLocatorService::new(waits), registry)
    }

    fn buttons_in_notes(position: MatchPosition) -> ElementTarget {
        ElementTarget::Locator(Locator {
            app: AppSelector::NameOrBundleId("Notes".to_owned()),
            query: ElementQuery::new(Some("button"), None, None).unwrap(),
            position,
            timeout: Duration::ZERO,
        })
    }

    fn notes_with_two_buttons(desktop: &FakeDesktop) {
        let process_id = desktop.add_app("Notes", None);
        let window = desktop.add_window(process_id, element("AXWindow", None));
        desktop.add_child(window, element("AXButton", Some("New")));
        desktop.add_child(window, element("AXButton", Some("Delete")));
    }

    #[test]
    fn passes_refs_through_unchanged() {
        let desktop = Arc::new(FakeDesktop::new());
        let (service, _) = locator_service(&desktop);
        let element_ref = ElementRef::from_index(42);

        assert_eq!(
            service.resolve(&ElementTarget::Ref(element_ref)),
            Ok(element_ref)
        );
    }

    #[test]
    fn resolves_the_nth_matching_element_to_a_registered_ref() {
        let desktop = Arc::new(FakeDesktop::new());
        notes_with_two_buttons(&desktop);
        let (service, registry) = locator_service(&desktop);

        let second_button = service
            .resolve(&buttons_in_notes(MatchPosition::Nth(1)))
            .unwrap();

        assert_eq!(second_button.to_string(), "e3");
        assert!(registry.resolve(&second_button).is_ok());
    }

    #[test]
    fn times_out_when_there_are_not_enough_matches() {
        let desktop = Arc::new(FakeDesktop::new());
        notes_with_two_buttons(&desktop);
        let (service, _) = locator_service(&desktop);

        let error = service
            .resolve(&buttons_in_notes(MatchPosition::Nth(5)))
            .unwrap_err();

        assert!(matches!(error, DeskwrightError::WaitTimedOut { .. }));
    }

    #[test]
    fn resolves_the_last_matching_element() {
        let desktop = Arc::new(FakeDesktop::new());
        notes_with_two_buttons(&desktop);
        let (service, _) = locator_service(&desktop);

        let last_button = service
            .resolve(&buttons_in_notes(MatchPosition::Last))
            .unwrap();

        assert_eq!(last_button.to_string(), "e3");
    }
}
