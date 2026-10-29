use std::sync::Arc;
use std::time::Duration;

use crate::element_registry::ElementRegistry;
use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::element::ElementRef;
use crate::model::native_element::NativeElement;
use crate::polling::wait_until;
use crate::ports::ElementTreeReader;
use crate::services::interaction::click_service::ClickService;

const TOGGLE_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// What [`ToggleService::set_checked`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToggleOutcome {
    AlreadyInState,
    Toggled { strategy_name: &'static str },
}

/// Checks or unchecks a checkbox, radio button or switch, like Playwright's `check()`: clicks
/// only when the state differs, then verifies the new state.
pub struct ToggleService {
    registry: Arc<ElementRegistry>,
    tree: Arc<dyn ElementTreeReader>,
    clicks: Arc<ClickService>,
    settle_timeout: Duration,
}

impl ToggleService {
    /// `settle_timeout` is how long to wait for the app to report the new state.
    pub fn new(
        registry: Arc<ElementRegistry>,
        tree: Arc<dyn ElementTreeReader>,
        clicks: Arc<ClickService>,
        settle_timeout: Duration,
    ) -> Self {
        Self {
            registry,
            tree,
            clicks,
            settle_timeout,
        }
    }

    pub fn set_checked(
        &self,
        element_ref: &ElementRef,
        should_be_checked: bool,
    ) -> DeskwrightResult<ToggleOutcome> {
        let element = self.registry.resolve(element_ref)?;
        if self.is_checked(&element)? == should_be_checked {
            return Ok(ToggleOutcome::AlreadyInState);
        }
        let outcome = self.clicks.click(element_ref)?;
        let has_settled = wait_until(self.settle_timeout, TOGGLE_POLL_INTERVAL, || {
            self.is_checked(&element).ok() == Some(should_be_checked)
        });
        if !has_settled {
            return Err(DeskwrightError::operation_failed(
                "change the checked state",
                "the element was clicked but its state did not change",
            ));
        }
        Ok(ToggleOutcome::Toggled {
            strategy_name: outcome.strategy_name,
        })
    }

    fn is_checked(&self, element: &NativeElement) -> DeskwrightResult<bool> {
        Ok(self.tree.attributes(element)?.is_checked())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::interaction::click_service::ClickStrategies;
    use crate::services::interaction::click_strategies::AccessibilityPressClick;
    use crate::testing::{FakeDesktop, FakeElementId, RecordedAction, element};

    fn toggle_service(desktop: &Arc<FakeDesktop>, registry: Arc<ElementRegistry>) -> ToggleService {
        let clicks = ClickService::new(
            registry.clone(),
            ClickStrategies {
                primary: vec![Box::new(AccessibilityPressClick::new(desktop.clone()))],
                double: Vec::new(),
                secondary: Vec::new(),
            },
        );
        ToggleService::new(registry, desktop.clone(), Arc::new(clicks), Duration::ZERO)
    }

    fn control(desktop: &FakeDesktop, role: &str, value: &str) -> FakeElementId {
        let process_id = desktop.add_app("Settings", None);
        let window = desktop.add_window(process_id, element("AXWindow", Some("Settings")));
        let control = desktop.add_child(window, element(role, Some("Wi-Fi")));
        desktop.set_value(control, value);
        control
    }

    #[test]
    fn checks_an_unchecked_checkbox_and_verifies_it() {
        let desktop = Arc::new(FakeDesktop::new());
        let checkbox = control(&desktop, "AXCheckBox", "0");
        let registry = Arc::new(ElementRegistry::default());
        let checkbox_ref = desktop.register(&registry, checkbox);

        let outcome = toggle_service(&desktop, registry)
            .set_checked(&checkbox_ref, true)
            .unwrap();

        assert_eq!(
            outcome,
            ToggleOutcome::Toggled {
                strategy_name: "accessibility press"
            }
        );
        assert_eq!(
            desktop.recorded_actions(),
            [RecordedAction::Press(checkbox)]
        );
    }

    #[test]
    fn does_nothing_when_already_in_the_requested_state() {
        let desktop = Arc::new(FakeDesktop::new());
        let checkbox = control(&desktop, "AXCheckBox", "1");
        let registry = Arc::new(ElementRegistry::default());
        let checkbox_ref = desktop.register(&registry, checkbox);

        let outcome = toggle_service(&desktop, registry)
            .set_checked(&checkbox_ref, true)
            .unwrap();

        assert_eq!(outcome, ToggleOutcome::AlreadyInState);
        assert!(desktop.recorded_actions().is_empty());
    }

    #[test]
    fn reports_a_click_that_did_not_change_the_state() {
        let desktop = Arc::new(FakeDesktop::new());
        let radio = control(&desktop, "AXRadioButton", "0");
        let registry = Arc::new(ElementRegistry::default());
        let radio_ref = desktop.register(&registry, radio);

        let error = toggle_service(&desktop, registry)
            .set_checked(&radio_ref, true)
            .unwrap_err();

        assert!(error.to_string().contains("state did not change"));
    }
}
