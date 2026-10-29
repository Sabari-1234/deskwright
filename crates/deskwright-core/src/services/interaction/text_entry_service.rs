use std::sync::Arc;

use crate::element_registry::ElementRegistry;
use crate::error::DeskwrightResult;
use crate::model::element::ElementRef;
use crate::services::interaction::strategy_chain::{InteractionOutcome, StrategyChain};
use crate::services::interaction::text_entry_strategies::TextEntryStrategy;

/// Enters text into an element from the latest snapshot using the first strategy that works.
pub struct TextEntryService {
    registry: Arc<ElementRegistry>,
    strategies: StrategyChain<dyn TextEntryStrategy>,
}

impl TextEntryService {
    /// `strategies` are tried in the given order.
    pub fn new(
        registry: Arc<ElementRegistry>,
        strategies: Vec<Box<dyn TextEntryStrategy>>,
    ) -> Self {
        Self {
            registry,
            strategies: StrategyChain::new("enter the text", strategies),
        }
    }

    pub fn enter_text(
        &self,
        element_ref: &ElementRef,
        text: &str,
    ) -> DeskwrightResult<InteractionOutcome> {
        let element = self.registry.resolve(element_ref)?;
        self.strategies
            .run_until_success(|strategy| strategy.enter_text(&element, text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::interaction::text_entry_strategies::{
        AccessibilityValueEntry, KeyboardTextEntry,
    };
    use crate::testing::{FakeDesktop, RecordedAction, element};

    fn text_entry_service(
        desktop: &Arc<FakeDesktop>,
        registry: Arc<ElementRegistry>,
    ) -> TextEntryService {
        TextEntryService::new(
            registry,
            vec![
                Box::new(AccessibilityValueEntry::new(
                    desktop.clone(),
                    desktop.clone(),
                )),
                Box::new(KeyboardTextEntry::new(
                    desktop.clone(),
                    desktop.clone(),
                    desktop.clone(),
                    std::time::Duration::ZERO,
                )),
            ],
        )
    }

    #[test]
    fn sets_the_value_through_accessibility_when_the_app_applies_it() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Notes", None);
        let field = desktop.add_window(process_id, element("AXTextField", None));
        let registry = Arc::new(ElementRegistry::default());
        let field_ref = desktop.register(&registry, field);

        let outcome = text_entry_service(&desktop, registry)
            .enter_text(&field_ref, "hello")
            .unwrap();

        assert_eq!(outcome.strategy_name, "accessibility value");
        assert!(
            desktop
                .recorded_actions()
                .contains(&RecordedAction::SetValue(field, "hello".to_owned()))
        );
    }

    #[test]
    fn falls_back_to_typing_when_the_app_ignores_the_new_value() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Slack", None);
        let field = desktop.add_window(process_id, element("AXTextArea", None));
        desktop.set_value(field, "");
        desktop.make_set_value_ignored(field);
        let registry = Arc::new(ElementRegistry::default());
        let field_ref = desktop.register(&registry, field);

        let outcome = text_entry_service(&desktop, registry)
            .enter_text(&field_ref, "hello")
            .unwrap();

        assert_eq!(outcome.strategy_name, "keyboard typing");
        assert_eq!(
            desktop.recorded_actions().last(),
            Some(&RecordedAction::TypeText("hello".to_owned()))
        );
    }

    #[test]
    fn types_into_non_editable_elements_instead_of_setting_their_value() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("WhatsApp", None);
        let search_box = desktop.add_window(process_id, element("AXStaticText", None));
        let registry = Arc::new(ElementRegistry::default());
        let search_ref = desktop.register(&registry, search_box);

        let outcome = text_entry_service(&desktop, registry)
            .enter_text(&search_ref, "ada")
            .unwrap();

        assert_eq!(outcome.strategy_name, "keyboard typing");
        assert!(
            !desktop
                .recorded_actions()
                .iter()
                .any(|action| matches!(action, RecordedAction::SetValue(..)))
        );
    }

    #[test]
    fn trusts_an_editable_field_whose_value_cannot_be_read_back() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("Safari", None);
        let password = desktop.add_window(process_id, element("AXTextField", Some("Password")));
        desktop.make_set_value_ignored(password);
        let registry = Arc::new(ElementRegistry::default());
        let password_ref = desktop.register(&registry, password);

        let outcome = text_entry_service(&desktop, registry)
            .enter_text(&password_ref, "secret")
            .unwrap();

        assert_eq!(outcome.strategy_name, "accessibility value");
    }

    #[test]
    fn presses_the_element_when_asking_for_focus_does_not_work() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("WhatsApp", None);
        let search_box = desktop.add_window(process_id, element("AXStaticText", None));
        desktop.make_focus_fail(search_box);
        let registry = Arc::new(ElementRegistry::default());
        let search_ref = desktop.register(&registry, search_box);

        let outcome = text_entry_service(&desktop, registry)
            .enter_text(&search_ref, "ada")
            .unwrap();

        assert_eq!(outcome.strategy_name, "keyboard typing");
        assert!(
            desktop
                .recorded_actions()
                .contains(&RecordedAction::Press(search_box))
        );
    }

    #[test]
    fn refuses_to_type_when_the_element_never_takes_focus() {
        let desktop = Arc::new(FakeDesktop::new());
        let process_id = desktop.add_app("WhatsApp", None);
        let label = desktop.add_window(process_id, element("AXStaticText", None));
        desktop.make_focus_fail(label);
        desktop.make_press_fail(label);
        let registry = Arc::new(ElementRegistry::default());
        let label_ref = desktop.register(&registry, label);

        let error = text_entry_service(&desktop, registry)
            .enter_text(&label_ref, "ada")
            .unwrap_err();

        assert!(error.to_string().contains("did not take keyboard focus"));
        assert!(
            !desktop
                .recorded_actions()
                .iter()
                .any(|action| matches!(action, RecordedAction::TypeText(_)))
        );
    }
}
