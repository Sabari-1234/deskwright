use std::sync::{Arc, Mutex, PoisonError};

use crate::element_registry::ElementRegistry;
use crate::error::DeskwrightResult;
use crate::model::app::AppSelector;
use crate::model::element_query::ElementQuery;
use crate::model::snapshot::AppSnapshot;
use crate::model::tree_depth::TreeDepth;
use crate::ports::{AccessibilityPermission, ElementTreeReader};
use crate::services::app_resolver::AppResolver;
use crate::services::tree_walker::{MatchGoal, TreeWalker};

/// Upper bound on elements per snapshot, so a huge app can't flood the client.
pub const MAX_SNAPSHOT_ELEMENTS: usize = 5_000;

/// Captures an app's element tree and registers its refs for later interaction.
pub struct SnapshotService {
    apps: AppResolver,
    permission: Arc<dyn AccessibilityPermission>,
    tree: Arc<dyn ElementTreeReader>,
    registry: Arc<ElementRegistry>,
    /// Serializes captures so two concurrent snapshots can't hand out the same ref numbers.
    capture_lock: Mutex<()>,
}

impl SnapshotService {
    pub fn new(
        apps: AppResolver,
        permission: Arc<dyn AccessibilityPermission>,
        tree: Arc<dyn ElementTreeReader>,
        registry: Arc<ElementRegistry>,
    ) -> Self {
        Self {
            apps,
            permission,
            tree,
            registry,
            capture_lock: Mutex::new(()),
        }
    }

    /// Reads the tree and replaces all previously issued refs with this snapshot's refs.
    pub fn capture(
        &self,
        selector: &AppSelector,
        depth: TreeDepth,
    ) -> DeskwrightResult<AppSnapshot> {
        self.capture_walk(selector, depth, None)
    }

    /// Like [`Self::capture`] at full depth, but stops reading once `needed_matches` elements
    /// match `query`. The rest of the tree is skipped, which makes searches much faster.
    pub fn capture_until_matches(
        &self,
        selector: &AppSelector,
        query: &ElementQuery,
        needed_matches: usize,
    ) -> DeskwrightResult<AppSnapshot> {
        let goal = MatchGoal {
            query: query.clone(),
            needed_matches,
        };
        self.capture_walk(selector, TreeDepth::MAXIMUM, Some(goal))
    }

    fn capture_walk(
        &self,
        selector: &AppSelector,
        depth: TreeDepth,
        goal: Option<MatchGoal>,
    ) -> DeskwrightResult<AppSnapshot> {
        self.permission.ensure_granted()?;
        // The guarded data is `()`, so a panic in another capture leaves nothing inconsistent.
        let _capture_guard = self
            .capture_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let app = self.apps.resolve(selector)?;
        let top_level_elements = self.tree.top_level_elements(app.process_id)?;
        let walked = TreeWalker::walk(
            self.tree.as_ref(),
            &top_level_elements,
            depth,
            MAX_SNAPSHOT_ELEMENTS,
            self.registry.next_index(),
            goal,
        );
        let element_count = walked.elements_by_ref.len();
        self.registry.replace_all(walked.elements_by_ref);
        Ok(AppSnapshot {
            app,
            elements: walked.roots,
            element_count,
            unreadable_element_count: walked.unreadable_element_count,
            is_truncated: walked.is_truncated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{DeskwrightError, Permission};
    use crate::model::element::ElementRef;
    use crate::testing::{FakeDesktop, element};

    fn service_for(desktop: &Arc<FakeDesktop>) -> (SnapshotService, Arc<ElementRegistry>) {
        let registry = Arc::new(ElementRegistry::default());
        let service = SnapshotService::new(
            AppResolver::new(desktop.clone()),
            desktop.clone(),
            desktop.clone(),
            registry.clone(),
        );
        (service, registry)
    }

    #[test]
    fn assigns_refs_depth_first_and_registers_them() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Notes", None);
        desktop.make_frontmost(process_id);
        let window = desktop.add_window(process_id, element("AXWindow", Some("Main")));
        let toolbar = desktop.add_child(window, element("AXToolbar", None));
        desktop.add_child(toolbar, element("AXButton", Some("New")));
        desktop.add_child(window, element("AXTextArea", None));
        let (service, registry) = service_for(&desktop);

        let snapshot = service
            .capture(&AppSelector::Frontmost, TreeDepth::DEFAULT)
            .unwrap();

        let refs: Vec<_> = snapshot.elements[0]
            .depth_first()
            .map(|node| node.summary.element_ref.to_string())
            .collect();
        assert_eq!(refs, ["e1", "e2", "e3", "e4"]);
        assert_eq!(snapshot.element_count, 4);
        assert!(registry.resolve(&ElementRef::from_index(4)).is_ok());
    }

    #[test]
    fn a_new_snapshot_continues_numbering_so_old_refs_stop_resolving() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Notes", None);
        desktop.make_frontmost(process_id);
        let window = desktop.add_window(process_id, element("AXWindow", None));
        desktop.add_child(window, element("AXButton", Some("Send")));
        let (service, registry) = service_for(&desktop);

        let first = service
            .capture(&AppSelector::Frontmost, TreeDepth::DEFAULT)
            .unwrap();
        let second = service
            .capture(&AppSelector::Frontmost, TreeDepth::DEFAULT)
            .unwrap();

        let first_button = first.elements[0].children[0].summary.element_ref;
        let second_button = second.elements[0].children[0].summary.element_ref;
        assert_eq!(first_button.to_string(), "e2");
        assert_eq!(second_button.to_string(), "e4");
        assert_eq!(
            registry.resolve(&first_button).unwrap_err(),
            DeskwrightError::UnknownElementRef(first_button)
        );
        assert!(registry.resolve(&second_button).is_ok());
    }

    #[test]
    fn stops_at_the_requested_depth() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Notes", None);
        let window = desktop.add_window(process_id, element("AXWindow", None));
        let group = desktop.add_child(window, element("AXGroup", None));
        desktop.add_child(group, element("AXButton", None));
        let (service, _) = service_for(&desktop);
        let selector = AppSelector::NameOrBundleId("Notes".to_owned());

        let snapshot = service
            .capture(&selector, TreeDepth::new(1).unwrap())
            .unwrap();

        assert_eq!(snapshot.element_count, 2);
        assert!(snapshot.elements[0].children[0].children.is_empty());
    }

    #[test]
    fn skips_unreadable_elements_and_counts_them() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Notes", None);
        let window = desktop.add_window(process_id, element("AXWindow", None));
        let vanished = desktop.add_child(window, element("AXButton", None));
        desktop.make_unreadable(vanished);
        let (service, _) = service_for(&desktop);
        let selector = AppSelector::NameOrBundleId("Notes".to_owned());

        let snapshot = service.capture(&selector, TreeDepth::DEFAULT).unwrap();

        assert_eq!(snapshot.element_count, 1);
        assert_eq!(snapshot.unreadable_element_count, 1);
    }

    #[test]
    fn fails_without_accessibility_permission() {
        let desktop = Arc::new(FakeDesktop::new());
        desktop.deny_accessibility_permission();
        let (service, _) = service_for(&desktop);

        let error = service
            .capture(&AppSelector::Frontmost, TreeDepth::DEFAULT)
            .unwrap_err();

        assert_eq!(
            error,
            DeskwrightError::PermissionDenied {
                permission: Permission::Accessibility
            }
        );
    }
}
