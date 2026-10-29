use deskwright_core::model::app::AppSelector;
use deskwright_core::model::element::ElementRef;
use deskwright_core::model::element_target::{ElementTarget, Locator, MatchPosition};
use deskwright_core::{DeskwrightError, DeskwrightResult};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::query_arguments::QueryArguments;
use crate::timeout_argument::timeout_from_seconds;

/// How a tool is told which element to act on: a `ref`, or a Playwright-style locator.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ElementTargetArguments {
    /// Element ref from the latest snapshot or find, e.g. "e12". Use either this or a locator.
    #[serde(rename = "ref")]
    pub element_ref: Option<String>,
    /// Locator criteria (any of role, title, value, placeholder, identifier, exact), and an
    /// optional `within` scope.
    #[serde(flatten)]
    pub query: QueryArguments,
    /// App name or bundle ID. Omit for the frontmost app.
    pub app: Option<String>,
    /// Locator: which match to use, 0 = first (default).
    pub nth: Option<usize>,
    /// Locator: use the last match instead of the nth.
    #[serde(default)]
    pub last: bool,
    /// Locator: how long to wait for the element, in seconds (default 5).
    pub timeout_seconds: Option<f64>,
}

impl ElementTargetArguments {
    /// `None` when neither a ref nor any locator criterion was given.
    pub fn to_target(&self) -> DeskwrightResult<Option<ElementTarget>> {
        match (&self.element_ref, self.query.has_criteria()) {
            (Some(_), true) => Err(DeskwrightError::InvalidInput(
                "give either ref or a locator (role/title/value/…), not both".to_owned(),
            )),
            (Some(raw_ref), false) => Ok(Some(ElementTarget::Ref(ElementRef::parse(raw_ref)?))),
            (None, true) => self
                .locator()
                .map(|locator| Some(ElementTarget::Locator(locator))),
            (None, false) => Ok(None),
        }
    }

    /// Like [`Self::to_target`], but an element is required.
    pub fn to_required_target(&self) -> DeskwrightResult<ElementTarget> {
        self.to_target()?.ok_or_else(|| {
            DeskwrightError::InvalidInput(
                "give a ref or a locator (role, title, value, placeholder or identifier) for the \
                 element"
                    .to_owned(),
            )
        })
    }

    /// A locator, refusing a `ref`: for checks that must search the live UI each time.
    pub fn to_required_locator(&self) -> DeskwrightResult<Locator> {
        if self.element_ref.is_some() {
            return Err(DeskwrightError::InvalidInput(
                "give a locator (role, title, value, placeholder or identifier), not a ref: a ref \
                 names one element from one snapshot"
                    .to_owned(),
            ));
        }
        if !self.query.has_criteria() {
            return Err(DeskwrightError::InvalidInput(
                "give a locator: at least one of role, title, value, placeholder or identifier"
                    .to_owned(),
            ));
        }
        self.locator()
    }

    pub fn app_selector(&self) -> AppSelector {
        AppSelector::from_optional_name(self.app.as_deref())
    }

    fn locator(&self) -> DeskwrightResult<Locator> {
        let position = match (self.nth, self.last) {
            (Some(_), true) => {
                return Err(DeskwrightError::InvalidInput(
                    "give either nth or last, not both".to_owned(),
                ));
            }
            (Some(nth), false) => MatchPosition::Nth(nth),
            (None, true) => MatchPosition::Last,
            (None, false) => MatchPosition::Nth(0),
        };
        Ok(Locator {
            app: self.app_selector(),
            query: self.query.to_query()?,
            position,
            timeout: timeout_from_seconds(self.timeout_seconds)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::query_arguments::CriteriaArguments;

    #[test]
    fn reads_a_ref() {
        let arguments = ElementTargetArguments {
            element_ref: Some("e7".to_owned()),
            ..ElementTargetArguments::default()
        };

        assert_eq!(
            arguments.to_target(),
            Ok(Some(ElementTarget::Ref(ElementRef::from_index(7))))
        );
    }

    #[test]
    fn builds_a_locator_with_defaults() {
        let arguments = ElementTargetArguments {
            query: QueryArguments {
                criteria: CriteriaArguments {
                    role: Some("button".to_owned()),
                    title: Some("Send".to_owned()),
                    ..CriteriaArguments::default()
                },
                within: None,
            },
            app: Some("WhatsApp".to_owned()),
            ..ElementTargetArguments::default()
        };

        let Some(ElementTarget::Locator(locator)) = arguments.to_target().unwrap() else {
            panic!("expected a locator");
        };

        assert_eq!(
            locator.app,
            AppSelector::NameOrBundleId("WhatsApp".to_owned())
        );
        assert_eq!(locator.position, MatchPosition::Nth(0));
        assert_eq!(locator.timeout, Duration::from_secs(5));
    }

    #[test]
    fn rejects_both_a_ref_and_a_locator() {
        let arguments = ElementTargetArguments {
            element_ref: Some("e7".to_owned()),
            query: QueryArguments {
                criteria: CriteriaArguments {
                    role: Some("button".to_owned()),
                    ..CriteriaArguments::default()
                },
                within: None,
            },
            ..ElementTargetArguments::default()
        };

        assert!(arguments.to_target().is_err());
    }

    #[test]
    fn reports_a_missing_target_only_when_one_is_required() {
        let nothing = ElementTargetArguments::default();

        assert_eq!(nothing.to_target(), Ok(None));
        assert!(nothing.to_required_target().is_err());
    }

    fn button_locator(nth: Option<usize>, last: bool) -> ElementTargetArguments {
        ElementTargetArguments {
            query: QueryArguments {
                criteria: CriteriaArguments {
                    role: Some("button".to_owned()),
                    ..CriteriaArguments::default()
                },
                within: None,
            },
            nth,
            last,
            ..ElementTargetArguments::default()
        }
    }

    #[test]
    fn picks_the_last_match_and_rejects_nth_with_last() {
        let Some(ElementTarget::Locator(locator)) = button_locator(None, true).to_target().unwrap()
        else {
            panic!("expected a locator");
        };

        assert_eq!(locator.position, MatchPosition::Last);
        assert!(button_locator(Some(1), true).to_target().is_err());
    }

    #[test]
    fn requires_a_locator_and_refuses_refs_when_asked() {
        let with_ref = ElementTargetArguments {
            element_ref: Some("e7".to_owned()),
            ..ElementTargetArguments::default()
        };

        assert!(with_ref.to_required_locator().is_err());
        assert!(
            ElementTargetArguments::default()
                .to_required_locator()
                .is_err()
        );
        assert_eq!(
            button_locator(None, true)
                .to_required_locator()
                .map(|locator| locator.position),
            Ok(MatchPosition::Last)
        );
    }
}
