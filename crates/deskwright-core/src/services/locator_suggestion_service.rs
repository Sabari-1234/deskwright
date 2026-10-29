use std::sync::Arc;

use crate::element_registry::ElementRegistry;
use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::app::AppSelector;
use crate::model::element::{ElementAttributes, ElementRef};
use crate::model::suggested_locator::SuggestedLocator;
use crate::ports::ElementTreeReader;
use crate::services::element_search_service::ElementSearchService;
use crate::services::snapshot_service::MAX_SNAPSHOT_ELEMENTS;

/// Turns a ref from a snapshot into a stable locator for flows and batches, like
/// Playwright's "pick locator": the most stable candidate that finds exactly that element,
/// or the first candidate that finds it with `nth` when none is unique.
pub struct LocatorSuggestionService {
    registry: Arc<ElementRegistry>,
    tree: Arc<dyn ElementTreeReader>,
    search: Arc<ElementSearchService>,
}

impl LocatorSuggestionService {
    pub fn new(
        registry: Arc<ElementRegistry>,
        tree: Arc<dyn ElementTreeReader>,
        search: Arc<ElementSearchService>,
    ) -> Self {
        Self {
            registry,
            tree,
            search,
        }
    }

    /// Checking candidates takes new snapshots, so `element_ref` (and every older ref) is
    /// stale afterwards.
    pub fn suggest(
        &self,
        element_ref: &ElementRef,
        app: &AppSelector,
    ) -> DeskwrightResult<SuggestedLocator> {
        let target = self.tree.attributes(&self.registry.resolve(element_ref)?)?;
        let mut fallback = None;
        for candidate in SuggestedLocator::candidates_for(&target) {
            let results = self
                .search
                .find(app, &candidate.to_query()?, MAX_SNAPSHOT_ELEMENTS)?;
            let Some(position) = results
                .matches
                .iter()
                .position(|found| is_same_element(&found.attributes, &target))
            else {
                continue;
            };
            if results.matches.len() == 1 {
                return Ok(candidate);
            }
            fallback.get_or_insert(SuggestedLocator {
                nth: Some(position),
                ..candidate
            });
        }
        fallback.ok_or_else(|| {
            DeskwrightError::operation_failed(
                "suggest a locator",
                format!(
                    "{element_ref} was not found in the app again; check `app` names the \
                     snapshot's app, or take a new snapshot"
                ),
            )
        })
    }
}

/// Elements found by different snapshots are compared by what identifies them on screen.
/// The value is left out because it can change between two reads.
fn is_same_element(found: &ElementAttributes, target: &ElementAttributes) -> bool {
    found.role == target.role
        && found.title == target.title
        && found.description == target.description
        && found.identifier == target.identifier
        && found.bounds == target.bounds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::geometry::Bounds;
    use crate::services::app_resolver::AppResolver;
    use crate::services::snapshot_service::SnapshotService;
    use crate::testing::{FakeDesktop, FakeElementId, element};

    fn service(
        desktop: &Arc<FakeDesktop>,
        registry: Arc<ElementRegistry>,
    ) -> LocatorSuggestionService {
        let snapshots = Arc::new(SnapshotService::new(
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            registry.clone(),
        ));
        LocatorSuggestionService::new(
            registry,
            desktop.clone(),
            Arc::new(ElementSearchService::new(snapshots)),
        )
    }

    fn button_at(
        desktop: &FakeDesktop,
        parent: FakeElementId,
        title: &str,
        x: f64,
    ) -> FakeElementId {
        let mut attributes = element("AXButton", Some(title));
        attributes.bounds = Some(Bounds {
            x,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        });
        desktop.add_child(parent, attributes)
    }

    fn chat_window(desktop: &FakeDesktop) -> FakeElementId {
        let process_id = desktop.add_app("WhatsApp", None);
        desktop.make_frontmost(process_id);
        desktop.add_window(process_id, element("AXWindow", Some("Chats")))
    }

    #[test]
    fn suggests_role_and_exact_title_when_they_are_unique() {
        let desktop = Arc::new(FakeDesktop::new());
        let window = chat_window(&desktop);
        button_at(&desktop, window, "Send", 0.0);
        let attach = button_at(&desktop, window, "Attach", 20.0);
        let registry = Arc::new(ElementRegistry::default());
        let attach_ref = desktop.register(&registry, attach);

        let locator = service(&desktop, registry)
            .suggest(&attach_ref, &AppSelector::Frontmost)
            .unwrap();

        assert_eq!(
            locator,
            SuggestedLocator {
                role: Some("button".to_owned()),
                title: Some("Attach".to_owned()),
                exact: true,
                ..SuggestedLocator::default()
            }
        );
    }

    #[test]
    fn adds_nth_when_no_candidate_is_unique() {
        let desktop = Arc::new(FakeDesktop::new());
        let window = chat_window(&desktop);
        button_at(&desktop, window, "Reply", 0.0);
        let second_reply = button_at(&desktop, window, "Reply", 20.0);
        let registry = Arc::new(ElementRegistry::default());
        let reply_ref = desktop.register(&registry, second_reply);

        let locator = service(&desktop, registry)
            .suggest(&reply_ref, &AppSelector::Frontmost)
            .unwrap();

        assert_eq!(locator.title.as_deref(), Some("Reply"));
        assert_eq!(locator.nth, Some(1));
    }

    #[test]
    fn prefers_a_unique_identifier() {
        let desktop = Arc::new(FakeDesktop::new());
        let window = chat_window(&desktop);
        let mut send = element("AXButton", Some("Send"));
        send.identifier = Some("compose-send".to_owned());
        let send = desktop.add_child(window, send);
        let registry = Arc::new(ElementRegistry::default());
        let send_ref = desktop.register(&registry, send);

        let locator = service(&desktop, registry)
            .suggest(&send_ref, &AppSelector::Frontmost)
            .unwrap();

        assert_eq!(locator.identifier.as_deref(), Some("compose-send"));
        assert_eq!(locator.role, None);
    }

    #[test]
    fn fails_when_the_element_is_in_another_app() {
        let desktop = Arc::new(FakeDesktop::new());
        let window = chat_window(&desktop);
        let send = button_at(&desktop, window, "Send", 0.0);
        desktop.add_app("Notes", None);
        let registry = Arc::new(ElementRegistry::default());
        let send_ref = desktop.register(&registry, send);

        let error = service(&desktop, registry)
            .suggest(&send_ref, &AppSelector::NameOrBundleId("Notes".to_owned()))
            .unwrap_err();

        assert!(error.to_string().contains("was not found in the app again"));
    }
}
