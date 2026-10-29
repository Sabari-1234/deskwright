use std::sync::Arc;

use crate::error::DeskwrightResult;
use crate::model::app::AppSelector;
use crate::model::element::ElementSummary;
use crate::model::element_query::ElementQuery;
use crate::services::snapshot_service::SnapshotService;

/// The first matches of a search, and whether the tree held more than were returned.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResults {
    pub matches: Vec<ElementSummary>,
    pub has_more: bool,
}

/// Finds elements matching a query anywhere in an app's tree.
pub struct ElementSearchService {
    snapshots: Arc<SnapshotService>,
}

impl ElementSearchService {
    pub fn new(snapshots: Arc<SnapshotService>) -> Self {
        Self { snapshots }
    }

    /// Returns up to `limit` matches in tree order. The walk stops one match past the limit,
    /// so it learns whether more exist without reading the rest of the tree. The returned
    /// refs are registered and usable.
    pub fn find(
        &self,
        selector: &AppSelector,
        query: &ElementQuery,
        limit: usize,
    ) -> DeskwrightResult<SearchResults> {
        let snapshot = self
            .snapshots
            .capture_until_matches(selector, query, limit + 1)?;
        let mut matches: Vec<ElementSummary> = query
            .matching_elements(&snapshot.elements)
            .into_iter()
            .cloned()
            .collect();
        let has_more = matches.len() > limit;
        matches.truncate(limit);
        Ok(SearchResults { matches, has_more })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element_registry::ElementRegistry;
    use crate::services::app_resolver::AppResolver;
    use crate::testing::{FakeDesktop, element};

    #[test]
    fn returns_only_matching_elements_across_the_tree() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Finder", None);
        desktop.make_frontmost(process_id);
        let window = desktop.add_window(process_id, element("AXWindow", Some("Downloads")));
        let toolbar = desktop.add_child(window, element("AXToolbar", None));
        desktop.add_child(toolbar, element("AXButton", Some("Back")));
        desktop.add_child(window, element("AXButton", Some("Share")));
        let snapshots = Arc::new(SnapshotService::new(
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            Arc::new(ElementRegistry::default()),
        ));
        let search = ElementSearchService::new(snapshots);
        let buttons = ElementQuery::new(Some("button"), None, None).unwrap();

        let results = search.find(&AppSelector::Frontmost, &buttons, 25).unwrap();

        assert!(!results.has_more);
        let titles: Vec<_> = results
            .matches
            .iter()
            .filter_map(|summary| summary.attributes.title.as_deref())
            .collect();
        assert_eq!(titles, ["Back", "Share"]);
    }

    fn search_over(desktop: &Arc<FakeDesktop>) -> ElementSearchService {
        ElementSearchService::new(Arc::new(SnapshotService::new(
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            Arc::new(ElementRegistry::default()),
        )))
    }

    #[test]
    fn stops_reading_the_tree_once_it_knows_there_are_more_matches() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Finder", None);
        desktop.make_frontmost(process_id);
        let window = desktop.add_window(process_id, element("AXWindow", None));
        for _ in 0..10 {
            desktop.add_child(window, element("AXButton", Some("Item")));
        }
        let query = ElementQuery::new(Some("button"), None, None).unwrap();

        let results = search_over(&desktop)
            .find(&AppSelector::Frontmost, &query, 1)
            .unwrap();

        assert_eq!(results.matches.len(), 1);
        assert!(results.has_more);
        assert_eq!(desktop.element_read_count(), 3);
    }

    #[test]
    fn scoped_searches_skip_matches_outside_the_scope_and_still_stop_early() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Mail", None);
        desktop.make_frontmost(process_id);
        let inbox = desktop.add_window(process_id, element("AXWindow", Some("Inbox")));
        desktop.add_child(inbox, element("AXButton", Some("Send")));
        let draft = desktop.add_window(process_id, element("AXWindow", Some("Draft")));
        desktop.add_child(draft, element("AXButton", Some("Send")));
        desktop.add_child(draft, element("AXButton", Some("Send later")));
        desktop.add_child(draft, element("AXButton", Some("Send copy")));
        let send_in_draft = ElementQuery::new(Some("button"), Some("send"), None)
            .unwrap()
            .within(ElementQuery::new(Some("window"), Some("draft"), None).unwrap());

        let results = search_over(&desktop)
            .find(&AppSelector::Frontmost, &send_in_draft, 1)
            .unwrap();

        assert_eq!(results.matches.len(), 1);
        assert_eq!(results.matches[0].element_ref.to_string(), "e4");
        assert!(results.has_more);
        assert_eq!(desktop.element_read_count(), 5);
    }
}
