use std::fmt;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::element::{ElementAttributes, ElementNode, ElementSummary};
use crate::model::visible_text::visible_text;

const ACCESSIBILITY_ROLE_PREFIX: &str = "ax";

/// Raw search criteria as a caller gives them; every field is optional.
#[derive(Debug, Clone, Copy, Default)]
pub struct QueryCriteria<'text> {
    /// Platform form (`AXButton`) or short form (`button`).
    pub role: Option<&'text str>,
    /// Matched against the title or the description.
    pub title: Option<&'text str>,
    pub value: Option<&'text str>,
    pub placeholder: Option<&'text str>,
    /// Always matched exactly: identifiers are stable keys, not display text.
    pub identifier: Option<&'text str>,
    /// Match title, value and placeholder exactly (ignoring case) instead of as substrings.
    pub is_exact: bool,
}

/// Criteria for finding elements. All given criteria must match; text matching ignores case.
/// The title criterion matches an element's title or its description, because many apps
/// (WhatsApp, most web content) label elements only through their description. With a scope,
/// only descendants of an element matching the scope count, like Playwright's
/// `locator.locator()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementQuery {
    normalized_role: Option<String>,
    title_text: Option<String>,
    value_text: Option<String>,
    placeholder_text: Option<String>,
    identifier: Option<String>,
    is_exact: bool,
    scope: Option<Box<ElementQuery>>,
}

impl ElementQuery {
    /// Shorthand for the common role/title/value case with substring matching.
    pub fn new(
        role: Option<&str>,
        title: Option<&str>,
        value: Option<&str>,
    ) -> DeskwrightResult<Self> {
        Self::from_criteria(QueryCriteria {
            role,
            title,
            value,
            ..QueryCriteria::default()
        })
    }

    pub fn from_criteria(criteria: QueryCriteria<'_>) -> DeskwrightResult<Self> {
        let query = Self {
            normalized_role: non_blank_lowercase(criteria.role)
                .map(|role| strip_role_prefix(&role)),
            title_text: non_blank_lowercase(criteria.title),
            value_text: non_blank_lowercase(criteria.value),
            placeholder_text: non_blank_lowercase(criteria.placeholder),
            identifier: criteria
                .identifier
                .map(str::trim)
                .filter(|identifier| !identifier.is_empty())
                .map(str::to_owned),
            is_exact: criteria.is_exact,
            scope: None,
        };
        if query.normalized_role.is_none()
            && query.title_text.is_none()
            && query.value_text.is_none()
            && query.placeholder_text.is_none()
            && query.identifier.is_none()
        {
            return Err(DeskwrightError::InvalidInput(
                "give at least one of role, title, value, placeholder or identifier to search for"
                    .to_owned(),
            ));
        }
        Ok(query)
    }

    /// Limits matches to descendants of elements matching `scope` (one level of scoping: the
    /// scope's own scope is dropped).
    pub fn within(mut self, mut scope: ElementQuery) -> Self {
        scope.scope = None;
        self.scope = Some(Box::new(scope));
        self
    }

    /// Whether descendants of an element with these attributes are inside the scope.
    pub fn opens_scope(&self, attributes: &ElementAttributes) -> bool {
        self.scope
            .as_ref()
            .is_some_and(|scope| scope.matches(attributes))
    }

    /// Whether an element matches, given whether it lies inside the scope. Without a scope,
    /// every element is inside it.
    pub fn matches_in_scope(&self, attributes: &ElementAttributes, is_inside_scope: bool) -> bool {
        (self.scope.is_none() || is_inside_scope) && self.matches(attributes)
    }

    /// The matching elements of a walked tree, in depth-first order, honoring the scope.
    pub fn matching_elements<'tree>(
        &self,
        roots: &'tree [ElementNode],
    ) -> Vec<&'tree ElementSummary> {
        let mut found = Vec::new();
        for root in roots {
            self.collect_matches(root, false, &mut found);
        }
        found
    }

    fn collect_matches<'tree>(
        &self,
        node: &'tree ElementNode,
        is_inside_scope: bool,
        found: &mut Vec<&'tree ElementSummary>,
    ) {
        let attributes = &node.summary.attributes;
        if self.matches_in_scope(attributes, is_inside_scope) {
            found.push(&node.summary);
        }
        let are_children_inside = is_inside_scope || self.opens_scope(attributes);
        for child in &node.children {
            self.collect_matches(child, are_children_inside, found);
        }
    }

    /// Whether the element's own attributes match, ignoring any scope.
    pub fn matches(&self, attributes: &ElementAttributes) -> bool {
        let role_matches = self.normalized_role.as_ref().is_none_or(|expected_role| {
            strip_role_prefix(&attributes.role.to_lowercase()) == *expected_role
        });
        let title_matches = self.title_text.as_deref().is_none_or(|expected| {
            self.text_matches(attributes.title.as_deref(), expected)
                || self.text_matches(attributes.description.as_deref(), expected)
        });
        let identifier_matches = self
            .identifier
            .as_deref()
            .is_none_or(|expected| attributes.identifier.as_deref() == Some(expected));
        role_matches
            && title_matches
            && identifier_matches
            && self.optional_text_matches(attributes.value.as_deref(), self.value_text.as_deref())
            && self.optional_text_matches(
                attributes.placeholder.as_deref(),
                self.placeholder_text.as_deref(),
            )
    }

    fn optional_text_matches(&self, actual: Option<&str>, expected: Option<&str>) -> bool {
        expected.is_none_or(|expected| self.text_matches(actual, expected))
    }

    /// `expected` is already lowercase. Invisible direction marks (which apps like WhatsApp
    /// put around labels) are ignored, so exact matching sees the text a person sees.
    fn text_matches(&self, actual: Option<&str>, expected: &str) -> bool {
        let Some(actual) = actual else {
            return false;
        };
        let visible = visible_lowercase(actual);
        if self.is_exact {
            visible == expected
        } else {
            visible.contains(expected)
        }
    }
}

impl fmt::Display for ElementQuery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let criteria: Vec<String> = [
            ("role", &self.normalized_role),
            ("title", &self.title_text),
            ("value", &self.value_text),
            ("placeholder", &self.placeholder_text),
            ("identifier", &self.identifier),
        ]
        .into_iter()
        .filter_map(|(name, criterion)| {
            criterion
                .as_ref()
                .map(|criterion| format!("{name} '{criterion}'"))
        })
        .collect();
        formatter.write_str(&criteria.join(", "))?;
        if self.is_exact {
            formatter.write_str(" (exact)")?;
        }
        if let Some(scope) = &self.scope {
            write!(formatter, " within ({scope})")?;
        }
        Ok(())
    }
}

fn visible_lowercase(text: &str) -> String {
    visible_text(text).to_lowercase()
}

fn non_blank_lowercase(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|trimmed| !trimmed.is_empty())
        .map(str::to_lowercase)
}

fn strip_role_prefix(lowercase_role: &str) -> String {
    lowercase_role
        .strip_prefix(ACCESSIBILITY_ROLE_PREFIX)
        .unwrap_or(lowercase_role)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button(title: &str) -> ElementAttributes {
        ElementAttributes {
            role: "AXButton".to_owned(),
            title: Some(title.to_owned()),
            ..ElementAttributes::default()
        }
    }

    #[test]
    fn rejects_a_query_without_criteria() {
        assert!(ElementQuery::new(None, Some("  "), None).is_err());
    }

    #[test]
    fn matches_role_in_short_or_platform_form_and_title_ignoring_case() {
        let short_form = ElementQuery::new(Some("button"), Some("SAVE"), None).unwrap();
        let platform_form = ElementQuery::new(Some("AXButton"), None, None).unwrap();

        assert!(short_form.matches(&button("Save As…")));
        assert!(platform_form.matches(&button("Cancel")));
        assert!(!short_form.matches(&button("Cancel")));
    }

    #[test]
    fn title_criterion_also_matches_the_description() {
        let query = ElementQuery::new(None, Some("ada"), None).unwrap();
        let labelled_only_by_description = ElementAttributes {
            role: "AXButton".to_owned(),
            description: Some("\u{200e}Ada Lovelace".to_owned()),
            ..ElementAttributes::default()
        };

        assert!(query.matches(&labelled_only_by_description));
        assert!(!query.matches(&button("Cancel")));
    }

    #[test]
    fn describes_its_criteria_for_messages() {
        let query = ElementQuery::new(Some("AXButton"), None, Some("Send")).unwrap();

        assert_eq!(query.to_string(), "role 'button', value 'send'");
    }

    #[test]
    fn value_criterion_fails_when_element_has_no_value() {
        let query = ElementQuery::new(None, None, Some("hello")).unwrap();
        assert!(!query.matches(&button("Hello")));
    }

    #[test]
    fn exact_matching_compares_whole_visible_text() {
        let exact = ElementQuery::from_criteria(QueryCriteria {
            title: Some("Ada Lovelace"),
            is_exact: true,
            ..QueryCriteria::default()
        })
        .unwrap();
        let chat_row = ElementAttributes {
            role: "AXButton".to_owned(),
            description: Some("\u{200e}Ada Lovelace".to_owned()),
            ..ElementAttributes::default()
        };
        let group_row = ElementAttributes {
            description: Some("Book Club, Ada Lovelace is also in this group".to_owned()),
            ..chat_row.clone()
        };

        assert!(exact.matches(&chat_row));
        assert!(!exact.matches(&group_row));
        assert_eq!(exact.to_string(), "title 'ada lovelace' (exact)");
    }

    #[test]
    fn matches_placeholder_and_identifier() {
        let field = ElementAttributes {
            role: "AXTextField".to_owned(),
            placeholder: Some("Search or start a new chat".to_owned()),
            identifier: Some("search-field".to_owned()),
            ..ElementAttributes::default()
        };
        let by_placeholder = ElementQuery::from_criteria(QueryCriteria {
            placeholder: Some("search"),
            ..QueryCriteria::default()
        })
        .unwrap();
        let by_identifier = ElementQuery::from_criteria(QueryCriteria {
            identifier: Some("search-field"),
            ..QueryCriteria::default()
        })
        .unwrap();
        let wrong_identifier = ElementQuery::from_criteria(QueryCriteria {
            identifier: Some("search"),
            ..QueryCriteria::default()
        })
        .unwrap();

        assert!(by_placeholder.matches(&field));
        assert!(by_identifier.matches(&field));
        assert!(!wrong_identifier.matches(&field));
    }

    fn node(attributes: ElementAttributes, children: Vec<ElementNode>) -> ElementNode {
        ElementNode {
            summary: ElementSummary {
                element_ref: crate::model::element::ElementRef::from_index(1),
                attributes,
            },
            children,
        }
    }

    #[test]
    fn scoped_queries_only_match_inside_the_scope() {
        let compose = node(
            ElementAttributes {
                role: "AXGroup".to_owned(),
                title: Some("Compose".to_owned()),
                ..ElementAttributes::default()
            },
            vec![node(button("Send"), Vec::new())],
        );
        let toolbar = node(
            ElementAttributes {
                role: "AXToolbar".to_owned(),
                ..ElementAttributes::default()
            },
            vec![node(button("Send"), Vec::new())],
        );
        let roots = [toolbar, compose];
        let send = ElementQuery::new(Some("button"), Some("send"), None).unwrap();
        let in_compose = send.clone().within(
            ElementQuery::new(Some("group"), Some("compose"), None)
                .unwrap()
                .within(ElementQuery::new(Some("window"), None, None).unwrap()),
        );

        assert_eq!(send.matching_elements(&roots).len(), 2);
        assert_eq!(in_compose.matching_elements(&roots).len(), 1);
        assert!(!in_compose.matches_in_scope(&button("Send"), false));
        assert_eq!(
            in_compose.to_string(),
            "role 'button', title 'send' within (role 'group', title 'compose')"
        );
    }
}
