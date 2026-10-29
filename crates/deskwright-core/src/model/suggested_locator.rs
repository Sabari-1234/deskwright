use serde::Serialize;

use crate::error::DeskwrightResult;
use crate::model::element::ElementAttributes;
use crate::model::element_query::{ElementQuery, QueryCriteria};

const ACCESSIBILITY_ROLE_PREFIX: &str = "AX";
/// AppKit gives elements without a developer-set identifier generated ones like `_NS:327`,
/// which change between app versions and launches, so they make poor locators.
const GENERATED_IDENTIFIER_PREFIX: &str = "_NS:";

/// A locator for one element in the shape the tools accept, as Playwright's "pick locator"
/// produces: the most stable criteria that find the element, plus `nth` when they match
/// several.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedLocator {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub exact: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nth: Option<usize>,
}

impl SuggestedLocator {
    /// Candidate locators for an element, most stable first: its identifier (set by
    /// developers, unaffected by language or content), then its role with its exact label or
    /// placeholder, then its role alone.
    pub fn candidates_for(attributes: &ElementAttributes) -> Vec<Self> {
        let role = Some(short_role(&attributes.role)).filter(|role| !role.is_empty());
        let label = non_empty(attributes.title.as_deref())
            .or_else(|| non_empty(attributes.description.as_deref()));
        let mut candidates = Vec::new();
        let stable_identifier = non_empty(attributes.identifier.as_deref())
            .filter(|identifier| !identifier.starts_with(GENERATED_IDENTIFIER_PREFIX));
        if let Some(identifier) = stable_identifier {
            candidates.push(Self {
                identifier: Some(identifier),
                ..Self::default()
            });
        }
        if let Some(label) = label {
            candidates.push(Self {
                role: role.clone(),
                title: Some(label),
                exact: true,
                ..Self::default()
            });
        }
        if let Some(placeholder) = non_empty(attributes.placeholder.as_deref()) {
            candidates.push(Self {
                role: role.clone(),
                placeholder: Some(placeholder),
                exact: true,
                ..Self::default()
            });
        }
        if role.is_some() {
            candidates.push(Self {
                role,
                ..Self::default()
            });
        }
        candidates
    }

    pub fn to_query(&self) -> DeskwrightResult<ElementQuery> {
        ElementQuery::from_criteria(QueryCriteria {
            role: self.role.as_deref(),
            title: self.title.as_deref(),
            placeholder: self.placeholder.as_deref(),
            identifier: self.identifier.as_deref(),
            is_exact: self.exact,
            ..QueryCriteria::default()
        })
    }
}

/// `AXButton` → `button`, the short form people write.
fn short_role(role: &str) -> String {
    role.strip_prefix(ACCESSIBILITY_ROLE_PREFIX)
        .unwrap_or(role)
        .to_lowercase()
}

fn non_empty(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes(role: &str) -> ElementAttributes {
        ElementAttributes {
            role: role.to_owned(),
            ..ElementAttributes::default()
        }
    }

    #[test]
    fn prefers_identifier_then_label_then_placeholder_then_role() {
        let field = ElementAttributes {
            identifier: Some("search-field".to_owned()),
            title: Some("Search".to_owned()),
            placeholder: Some("Search or start a new chat".to_owned()),
            ..attributes("AXTextField")
        };

        let candidates = SuggestedLocator::candidates_for(&field);

        assert_eq!(
            candidates,
            [
                SuggestedLocator {
                    identifier: Some("search-field".to_owned()),
                    ..SuggestedLocator::default()
                },
                SuggestedLocator {
                    role: Some("textfield".to_owned()),
                    title: Some("Search".to_owned()),
                    exact: true,
                    ..SuggestedLocator::default()
                },
                SuggestedLocator {
                    role: Some("textfield".to_owned()),
                    placeholder: Some("Search or start a new chat".to_owned()),
                    exact: true,
                    ..SuggestedLocator::default()
                },
                SuggestedLocator {
                    role: Some("textfield".to_owned()),
                    ..SuggestedLocator::default()
                },
            ]
        );
    }

    #[test]
    fn skips_identifiers_generated_by_appkit() {
        let checkbox = ElementAttributes {
            identifier: Some("_NS:320".to_owned()),
            title: Some("Show item info".to_owned()),
            ..attributes("AXCheckBox")
        };

        let candidates = SuggestedLocator::candidates_for(&checkbox);

        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.identifier.is_none())
        );
        assert_eq!(candidates[0].title.as_deref(), Some("Show item info"));
    }

    #[test]
    fn uses_the_description_when_there_is_no_title() {
        let button = ElementAttributes {
            description: Some("Send".to_owned()),
            ..attributes("AXButton")
        };

        let candidates = SuggestedLocator::candidates_for(&button);

        assert_eq!(candidates[0].title.as_deref(), Some("Send"));
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn builds_queries_and_serializes_only_what_is_set() {
        let locator = SuggestedLocator {
            role: Some("button".to_owned()),
            title: Some("Send".to_owned()),
            exact: true,
            nth: Some(1),
            ..SuggestedLocator::default()
        };

        assert_eq!(
            locator.to_query().unwrap().to_string(),
            "role 'button', title 'send' (exact)"
        );
        assert_eq!(
            serde_json::to_string(&locator).unwrap(),
            r#"{"role":"button","title":"Send","exact":true,"nth":1}"#
        );
        assert!(SuggestedLocator::candidates_for(&attributes("")).is_empty());
    }
}
