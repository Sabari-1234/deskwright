use std::sync::Arc;

use crate::element_registry::ElementRegistry;
use crate::error::DeskwrightResult;
use crate::model::element::ElementRef;
use crate::model::element_target::ElementTarget;
use crate::model::native_element::NativeElement;
use crate::ports::InputSynthesizer;
use crate::services::element_locator_service::ElementLocatorService;
use crate::services::interaction::pointer_target::PointerTarget;

/// Mouse gestures with no accessibility equivalent: hovering over an element and dragging
/// one element onto another.
pub struct PointerGestureService {
    registry: Arc<ElementRegistry>,
    locator: Arc<ElementLocatorService>,
    target: PointerTarget,
    input: Arc<dyn InputSynthesizer>,
}

impl PointerGestureService {
    pub fn new(
        registry: Arc<ElementRegistry>,
        locator: Arc<ElementLocatorService>,
        target: PointerTarget,
        input: Arc<dyn InputSynthesizer>,
    ) -> Self {
        Self {
            registry,
            locator,
            target,
            input,
        }
    }

    /// Moves the pointer over the element's center, revealing tooltips and hover states.
    pub fn hover(&self, element_ref: &ElementRef) -> DeskwrightResult<()> {
        let element = self.registry.resolve(element_ref)?;
        let point = self.target.safe_point(&element)?;
        self.input.move_pointer_to(point)
    }

    /// Drags `source` onto `destination`. The destination may belong to another app (dropping
    /// a file from Finder into Mail), so only the source is checked for covering windows.
    pub fn drag(
        &self,
        source: &ElementTarget,
        destination: &ElementTarget,
    ) -> DeskwrightResult<()> {
        // Locating the destination takes a new snapshot, which invalidates the source's ref,
        // so the source element is resolved before that happens.
        let source_element = self.locate(source)?;
        let destination_element = self.locate(destination)?;
        let from = self.target.safe_point(&source_element)?;
        let to = self.target.visible_center(&destination_element)?;
        self.input.drag(from, to)
    }

    fn locate(&self, target: &ElementTarget) -> DeskwrightResult<NativeElement> {
        let element_ref = self.locator.resolve(target)?;
        self.registry.resolve(&element_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::model::app::AppSelector;
    use crate::model::element_query::ElementQuery;
    use crate::model::element_target::{Locator, MatchPosition};
    use crate::model::geometry::{Bounds, Point};
    use crate::services::app_resolver::AppResolver;
    use crate::services::element_search_service::ElementSearchService;
    use crate::services::element_wait_service::ElementWaitService;
    use crate::services::snapshot_service::SnapshotService;
    use crate::testing::{FakeDesktop, FakeElementId, RecordedAction, element};

    fn visible(desktop: &FakeDesktop, parent: FakeElementId, x: f64, title: &str) -> FakeElementId {
        let mut attributes = element("AXCell", Some(title));
        attributes.bounds = Some(Bounds {
            x,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        });
        desktop.add_child(parent, attributes)
    }

    fn gestures(
        desktop: &Arc<FakeDesktop>,
        registry: Arc<ElementRegistry>,
    ) -> PointerGestureService {
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
        PointerGestureService::new(
            registry,
            Arc::new(ElementLocatorService::new(waits)),
            PointerTarget::new(desktop.clone(), desktop.clone()),
            desktop.clone(),
        )
    }

    fn by_ref(element_ref: ElementRef) -> ElementTarget {
        ElementTarget::Ref(element_ref)
    }

    fn board(desktop: &FakeDesktop) -> (FakeElementId, FakeElementId) {
        let process_id = desktop.add_app("Trello", None);
        let window = desktop.add_window(process_id, element("AXWindow", Some("Board")));
        (
            visible(desktop, window, 0.0, "Card"),
            visible(desktop, window, 100.0, "Done"),
        )
    }

    #[test]
    fn hovers_over_the_element_center() {
        let desktop = Arc::new(FakeDesktop::new());
        let (card, _) = board(&desktop);
        let registry = Arc::new(ElementRegistry::default());
        let card_ref = desktop.register(&registry, card);

        gestures(&desktop, registry).hover(&card_ref).unwrap();

        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::MovePointerTo(Point { x: 5.0, y: 5.0 })]
        );
    }

    #[test]
    fn drags_from_one_center_to_the_other() {
        let desktop = Arc::new(FakeDesktop::new());
        let (card, done) = board(&desktop);
        let registry = Arc::new(ElementRegistry::default());
        let refs = desktop.register_all(&registry, &[card, done]);

        gestures(&desktop, registry)
            .drag(&by_ref(refs[0]), &by_ref(refs[1]))
            .unwrap();

        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::Drag {
                from: Point { x: 5.0, y: 5.0 },
                to: Point { x: 105.0, y: 5.0 },
            }]
        );
    }

    #[test]
    fn refuses_to_hover_over_a_covered_element() {
        let desktop = Arc::new(FakeDesktop::new());
        let (card, _) = board(&desktop);
        desktop.make_covered(card);
        let registry = Arc::new(ElementRegistry::default());
        let card_ref = desktop.register(&registry, card);

        let error = gestures(&desktop, registry).hover(&card_ref).unwrap_err();

        assert!(error.to_string().contains("covers it"));
        assert!(desktop.recorded_actions().is_empty());
    }

    #[test]
    fn refuses_to_drag_to_an_invisible_destination() {
        let desktop = Arc::new(FakeDesktop::new());
        let (card, _) = board(&desktop);
        let process_id = desktop.add_app("Finder", None);
        let hidden = desktop.add_window(process_id, element("AXWindow", Some("Hidden")));
        let registry = Arc::new(ElementRegistry::default());
        let refs = desktop.register_all(&registry, &[card, hidden]);

        let error = gestures(&desktop, registry)
            .drag(&by_ref(refs[0]), &by_ref(refs[1]))
            .unwrap_err();

        assert!(error.to_string().contains("not visible"));
    }

    #[test]
    fn keeps_the_source_when_locating_the_destination_takes_a_new_snapshot() {
        let desktop = Arc::new(FakeDesktop::new());
        let (card, _) = board(&desktop);
        let registry = Arc::new(ElementRegistry::default());
        let card_ref = desktop.register(&registry, card);
        let done_locator = ElementTarget::Locator(Locator {
            app: AppSelector::NameOrBundleId("Trello".to_owned()),
            query: ElementQuery::new(None, Some("Done"), None).unwrap(),
            position: MatchPosition::Nth(0),
            timeout: Duration::ZERO,
        });

        gestures(&desktop, registry)
            .drag(&by_ref(card_ref), &done_locator)
            .unwrap();

        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::Drag {
                from: Point { x: 5.0, y: 5.0 },
                to: Point { x: 105.0, y: 5.0 },
            }]
        );
    }
}
