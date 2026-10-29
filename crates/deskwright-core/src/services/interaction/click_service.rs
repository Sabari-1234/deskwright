use std::sync::Arc;

use crate::element_registry::ElementRegistry;
use crate::error::DeskwrightResult;
use crate::model::element::ElementRef;
use crate::services::interaction::click_strategies::ClickStrategy;
use crate::services::interaction::strategy_chain::{InteractionOutcome, StrategyChain};

/// What kind of click to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClickKind {
    #[default]
    Primary,
    Double,
    /// A right-click, usually opening a context menu.
    Secondary,
}

/// The ordered strategies for each kind of click.
pub struct ClickStrategies {
    pub primary: Vec<Box<dyn ClickStrategy>>,
    pub double: Vec<Box<dyn ClickStrategy>>,
    pub secondary: Vec<Box<dyn ClickStrategy>>,
}

/// Clicks an element from the latest snapshot using the first strategy that works for the
/// requested kind of click.
pub struct ClickService {
    registry: Arc<ElementRegistry>,
    primary: StrategyChain<dyn ClickStrategy>,
    double: StrategyChain<dyn ClickStrategy>,
    secondary: StrategyChain<dyn ClickStrategy>,
}

impl ClickService {
    /// Each kind's strategies are tried in the given order.
    pub fn new(registry: Arc<ElementRegistry>, strategies: ClickStrategies) -> Self {
        Self {
            registry,
            primary: StrategyChain::new("click the element", strategies.primary),
            double: StrategyChain::new("double-click the element", strategies.double),
            secondary: StrategyChain::new("right-click the element", strategies.secondary),
        }
    }

    pub fn click(&self, element_ref: &ElementRef) -> DeskwrightResult<InteractionOutcome> {
        self.click_with(element_ref, ClickKind::Primary)
    }

    pub fn click_with(
        &self,
        element_ref: &ElementRef,
        kind: ClickKind,
    ) -> DeskwrightResult<InteractionOutcome> {
        let element = self.registry.resolve(element_ref)?;
        let chain = match kind {
            ClickKind::Primary => &self.primary,
            ClickKind::Double => &self.double,
            ClickKind::Secondary => &self.secondary,
        };
        chain.run_until_success(|strategy| strategy.click(&element))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::geometry::{Bounds, Point};
    use crate::services::interaction::click_strategies::{
        AccessibilityMenuClick, AccessibilityPressClick, PointerClick, PointerGesture,
    };
    use crate::services::interaction::pointer_target::PointerTarget;
    use crate::testing::{FakeDesktop, FakeElementId, RecordedAction, element};

    fn click_service(desktop: &Arc<FakeDesktop>, registry: Arc<ElementRegistry>) -> ClickService {
        let target = PointerTarget::new(desktop.clone(), desktop.clone());
        let pointer = |gesture| -> Box<dyn ClickStrategy> {
            Box::new(PointerClick::new(target.clone(), desktop.clone(), gesture))
        };
        ClickService::new(
            registry,
            ClickStrategies {
                primary: vec![
                    Box::new(AccessibilityPressClick::new(desktop.clone())),
                    pointer(PointerGesture::Single),
                ],
                double: vec![pointer(PointerGesture::Double)],
                secondary: vec![
                    Box::new(AccessibilityMenuClick::new(desktop.clone())),
                    pointer(PointerGesture::Secondary),
                ],
            },
        )
    }

    #[test]
    fn presses_through_accessibility_when_supported() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Notes", None);
        let button = desktop.add_window(process_id, element("AXButton", Some("OK")));
        let registry = Arc::new(ElementRegistry::default());
        let button_ref = desktop.register(&registry, button);

        let outcome = click_service(&desktop, registry)
            .click(&button_ref)
            .unwrap();

        assert_eq!(outcome.strategy_name, "accessibility press");
        assert_eq!(desktop.recorded_actions(), [RecordedAction::Press(button)]);
    }

    #[test]
    fn falls_back_to_a_pointer_click_at_the_element_center() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Notes", None);
        let mut attributes = element("AXGroup", Some("Canvas"));
        attributes.bounds = Some(Bounds {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 40.0,
        });
        let canvas = desktop.add_window(process_id, attributes);
        desktop.make_press_fail(canvas);
        let registry = Arc::new(ElementRegistry::default());
        let canvas_ref = desktop.register(&registry, canvas);

        let outcome = click_service(&desktop, registry)
            .click(&canvas_ref)
            .unwrap();

        assert_eq!(outcome.strategy_name, "pointer click");
        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::ClickAt(Point { x: 60.0, y: 40.0 })]
        );
    }

    #[test]
    fn refuses_a_pointer_click_on_an_element_covered_by_another_app() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Notes", None);
        let mut attributes = element("AXTextArea", None);
        attributes.bounds = Some(Bounds {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        });
        let text_area = desktop.add_window(process_id, attributes);
        desktop.make_press_fail(text_area);
        desktop.make_covered(text_area);
        let registry = Arc::new(ElementRegistry::default());
        let text_area_ref = desktop.register(&registry, text_area);

        let error = click_service(&desktop, registry)
            .click(&text_area_ref)
            .unwrap_err();

        assert!(error.to_string().contains("another app's window covers it"));
        assert!(desktop.recorded_actions().is_empty());
    }

    #[test]
    fn fails_when_no_strategy_can_click() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Notes", None);
        let hidden = desktop.add_window(process_id, element("AXGroup", None));
        desktop.make_press_fail(hidden);
        let registry = Arc::new(ElementRegistry::default());
        let hidden_ref = desktop.register(&registry, hidden);

        let error = click_service(&desktop, registry)
            .click(&hidden_ref)
            .unwrap_err();

        assert!(error.to_string().contains("every strategy failed"));
    }

    fn visible_element(desktop: &FakeDesktop, role: &str) -> FakeElementId {
        let process_id = desktop.add_app("Finder", None);
        let mut attributes = element(role, Some("Item"));
        attributes.bounds = Some(Bounds {
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 10.0,
        });
        desktop.add_window(process_id, attributes)
    }

    #[test]
    fn double_clicks_with_the_pointer_at_the_center() {
        let desktop = Arc::new(FakeDesktop::new());
        let file = visible_element(&desktop, "AXCell");
        let registry = Arc::new(ElementRegistry::default());
        let file_ref = desktop.register(&registry, file);

        let outcome = click_service(&desktop, registry)
            .click_with(&file_ref, ClickKind::Double)
            .unwrap();

        assert_eq!(outcome.strategy_name, "pointer double-click");
        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::DoubleClickAt(Point { x: 10.0, y: 5.0 })]
        );
    }

    #[test]
    fn right_click_opens_the_menu_through_accessibility_first() {
        let desktop = Arc::new(FakeDesktop::new());
        let file = visible_element(&desktop, "AXCell");
        let registry = Arc::new(ElementRegistry::default());
        let file_ref = desktop.register(&registry, file);

        let outcome = click_service(&desktop, registry)
            .click_with(&file_ref, ClickKind::Secondary)
            .unwrap();

        assert_eq!(outcome.strategy_name, "accessibility menu");
        assert_eq!(desktop.recorded_actions(), [RecordedAction::ShowMenu(file)]);
    }

    #[test]
    fn right_click_falls_back_to_the_pointer() {
        let desktop = Arc::new(FakeDesktop::new());
        let file = visible_element(&desktop, "AXCell");
        desktop.make_show_menu_fail(file);
        let registry = Arc::new(ElementRegistry::default());
        let file_ref = desktop.register(&registry, file);

        let outcome = click_service(&desktop, registry)
            .click_with(&file_ref, ClickKind::Secondary)
            .unwrap();

        assert_eq!(outcome.strategy_name, "pointer right-click");
        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::RightClickAt(Point { x: 10.0, y: 5.0 })]
        );
    }
}
