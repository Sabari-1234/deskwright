//! States that the elements matching a query can be expected to reach, like Playwright's
//! `expect(locator).toBeVisible()`. Each condition is one small type (**Strategy**), so a new
//! assertion is a new type, not a new branch.

use crate::model::element::ElementSummary;

/// What one check of a query saw: every match, and the one the expectation is about.
#[derive(Debug, Clone, Copy)]
pub struct Observation<'seen> {
    pub matches: &'seen [ElementSummary],
    pub selected: Option<&'seen ElementSummary>,
}

/// A state of a query's matches, checked repeatedly until it holds.
pub trait ElementCondition: Send + Sync {
    /// The expected state, completing "expected <subject> …", like "to be enabled".
    fn description(&self) -> String;

    /// `Ok` when the state holds; otherwise what was seen instead, completing "but …".
    fn check(&self, observation: &Observation<'_>) -> Result<(), String>;

    /// Whether the check needs every match (a count) rather than just the selected one.
    fn needs_every_match(&self) -> bool {
        false
    }
}

const NOT_FOUND: &str = "no element matched";

/// Applies `is_met` to the selected element; a missing element never meets it.
fn check_selected(
    observation: &Observation<'_>,
    is_met: impl Fn(&ElementSummary) -> bool,
    describe_unmet: impl Fn(&ElementSummary) -> String,
) -> Result<(), String> {
    match observation.selected {
        None => Err(NOT_FOUND.to_owned()),
        Some(element) if is_met(element) => Ok(()),
        Some(element) => Err(format!(
            "{} {}",
            element.element_ref,
            describe_unmet(element)
        )),
    }
}

/// The element exists and has a visible area on screen.
pub struct IsVisible;

impl ElementCondition for IsVisible {
    fn description(&self) -> String {
        "to be visible".to_owned()
    }

    fn check(&self, observation: &Observation<'_>) -> Result<(), String> {
        check_selected(
            observation,
            |element| element.attributes.is_visible(),
            |_| "has no visible area".to_owned(),
        )
    }
}

/// No matching element is visible: it is gone, or has no visible area.
pub struct IsHidden;

impl ElementCondition for IsHidden {
    fn description(&self) -> String {
        "to be hidden".to_owned()
    }

    fn check(&self, observation: &Observation<'_>) -> Result<(), String> {
        match observation.selected {
            Some(element) if element.attributes.is_visible() => {
                Err(format!("{} is still visible", element.element_ref))
            }
            _ => Ok(()),
        }
    }
}

/// The element is enabled (`true`) or disabled (`false`).
pub struct IsEnabled(pub bool);

impl ElementCondition for IsEnabled {
    fn description(&self) -> String {
        if self.0 {
            "to be enabled"
        } else {
            "to be disabled"
        }
        .to_owned()
    }

    fn check(&self, observation: &Observation<'_>) -> Result<(), String> {
        check_selected(
            observation,
            |element| element.attributes.is_enabled == self.0,
            |element| {
                if element.attributes.is_enabled {
                    "is enabled".to_owned()
                } else {
                    "is disabled".to_owned()
                }
            },
        )
    }
}

/// The checkbox, radio button or switch is checked (`true`) or unchecked (`false`).
pub struct IsChecked(pub bool);

impl ElementCondition for IsChecked {
    fn description(&self) -> String {
        if self.0 {
            "to be checked"
        } else {
            "to be unchecked"
        }
        .to_owned()
    }

    fn check(&self, observation: &Observation<'_>) -> Result<(), String> {
        check_selected(
            observation,
            |element| element.attributes.is_checked() == self.0,
            |element| {
                if element.attributes.is_checked() {
                    "is checked".to_owned()
                } else {
                    "is unchecked".to_owned()
                }
            },
        )
    }
}

/// The element has keyboard focus.
pub struct IsFocused;

impl ElementCondition for IsFocused {
    fn description(&self) -> String {
        "to be focused".to_owned()
    }

    fn check(&self, observation: &Observation<'_>) -> Result<(), String> {
        check_selected(
            observation,
            |element| element.attributes.is_focused,
            |_| "does not have focus".to_owned(),
        )
    }
}

/// The element's value (or, when it has none, its title) equals or contains `text`.
pub struct HasText {
    pub text: String,
    pub is_exact: bool,
}

impl HasText {
    fn shown_text(element: &ElementSummary) -> &str {
        let attributes = &element.attributes;
        attributes
            .value
            .as_deref()
            .or(attributes.title.as_deref())
            .unwrap_or_default()
    }
}

impl ElementCondition for HasText {
    fn description(&self) -> String {
        if self.is_exact {
            format!("to have the text \"{}\"", self.text)
        } else {
            format!("to contain the text \"{}\"", self.text)
        }
    }

    fn check(&self, observation: &Observation<'_>) -> Result<(), String> {
        check_selected(
            observation,
            |element| {
                let shown = Self::shown_text(element);
                if self.is_exact {
                    shown == self.text
                } else {
                    shown.to_lowercase().contains(&self.text.to_lowercase())
                }
            },
            |element| format!("shows \"{}\"", Self::shown_text(element)),
        )
    }
}

/// Exactly this many elements match.
pub struct HasCount(pub usize);

impl ElementCondition for HasCount {
    fn description(&self) -> String {
        format!("to match {} elements", self.0)
    }

    fn check(&self, observation: &Observation<'_>) -> Result<(), String> {
        let count = observation.matches.len();
        if count == self.0 {
            Ok(())
        } else {
            Err(format!("{count} matched"))
        }
    }

    fn needs_every_match(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::element::{ElementAttributes, ElementRef};
    use crate::model::geometry::Bounds;

    fn summary(index: usize, attributes: ElementAttributes) -> ElementSummary {
        ElementSummary {
            element_ref: ElementRef::from_index(index),
            attributes,
        }
    }

    fn visible_button() -> ElementSummary {
        summary(
            4,
            ElementAttributes {
                role: "AXButton".to_owned(),
                title: Some("Send".to_owned()),
                is_enabled: true,
                bounds: Some(Bounds {
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 10.0,
                }),
                ..ElementAttributes::default()
            },
        )
    }

    fn observe(matches: &[ElementSummary]) -> Observation<'_> {
        Observation {
            matches,
            selected: matches.first(),
        }
    }

    #[test]
    fn visibility_needs_an_element_with_a_visible_area() {
        let button = [visible_button()];
        let invisible = [summary(5, ElementAttributes::default())];

        assert_eq!(IsVisible.check(&observe(&button)), Ok(()));
        assert_eq!(
            IsVisible.check(&observe(&invisible)),
            Err("e5 has no visible area".to_owned())
        );
        assert_eq!(
            IsVisible.check(&observe(&[])),
            Err("no element matched".to_owned())
        );
        assert_eq!(IsHidden.check(&observe(&[])), Ok(()));
        assert_eq!(IsHidden.check(&observe(&invisible)), Ok(()));
        assert_eq!(
            IsHidden.check(&observe(&button)),
            Err("e4 is still visible".to_owned())
        );
    }

    #[test]
    fn checks_enabled_and_disabled() {
        let mut disabled = visible_button();
        disabled.attributes.is_enabled = false;
        let enabled = [visible_button()];
        let disabled = [disabled];

        assert_eq!(IsEnabled(true).check(&observe(&enabled)), Ok(()));
        assert_eq!(
            IsEnabled(true).check(&observe(&disabled)),
            Err("e4 is disabled".to_owned())
        );
        assert_eq!(
            IsEnabled(false).check(&observe(&enabled)),
            Err("e4 is enabled".to_owned())
        );
        assert_eq!(IsEnabled(false).description(), "to be disabled");
    }

    #[test]
    fn checks_checked_state_and_focus() {
        let mut checkbox = visible_button();
        checkbox.attributes.value = Some("1".to_owned());
        checkbox.attributes.is_focused = true;
        let checked = [checkbox];
        let plain = [visible_button()];

        assert_eq!(IsChecked(true).check(&observe(&checked)), Ok(()));
        assert_eq!(
            IsChecked(false).check(&observe(&checked)),
            Err("e4 is checked".to_owned())
        );
        assert_eq!(
            IsChecked(true).check(&observe(&plain)),
            Err("e4 is unchecked".to_owned())
        );
        assert_eq!(IsChecked(false).description(), "to be unchecked");
        assert_eq!(IsFocused.check(&observe(&checked)), Ok(()));
        assert_eq!(
            IsFocused.check(&observe(&plain)),
            Err("e4 does not have focus".to_owned())
        );
    }

    #[test]
    fn matches_text_exactly_or_as_a_substring() {
        let mut field = visible_button();
        field.attributes.value = Some("Hello World".to_owned());
        let field = [field];
        let contains = HasText {
            text: "hello".to_owned(),
            is_exact: false,
        };
        let exact = HasText {
            text: "hello".to_owned(),
            is_exact: true,
        };

        assert_eq!(contains.check(&observe(&field)), Ok(()));
        assert_eq!(
            exact.check(&observe(&field)),
            Err("e4 shows \"Hello World\"".to_owned())
        );
        assert_eq!(exact.description(), "to have the text \"hello\"");
        assert_eq!(contains.description(), "to contain the text \"hello\"");
    }

    #[test]
    fn falls_back_to_the_title_for_text() {
        let button = [visible_button()];
        let condition = HasText {
            text: "Send".to_owned(),
            is_exact: true,
        };

        assert_eq!(condition.check(&observe(&button)), Ok(()));
    }

    #[test]
    fn counts_every_match() {
        let two = [visible_button(), visible_button()];

        assert!(HasCount(2).needs_every_match());
        assert!(!IsVisible.needs_every_match());
        assert_eq!(HasCount(2).check(&observe(&two)), Ok(()));
        assert_eq!(
            HasCount(1).check(&observe(&two)),
            Err("2 matched".to_owned())
        );
        assert_eq!(HasCount(1).description(), "to match 1 elements");
        assert_eq!(IsFocused.description(), "to be focused");
        assert_eq!(IsVisible.description(), "to be visible");
        assert_eq!(IsHidden.description(), "to be hidden");
        assert_eq!(IsEnabled(true).description(), "to be enabled");
        assert_eq!(IsChecked(true).description(), "to be checked");
    }
}
