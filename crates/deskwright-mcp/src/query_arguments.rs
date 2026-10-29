use deskwright_core::DeskwrightResult;
use deskwright_core::model::element_query::{ElementQuery, QueryCriteria};
use schemars::JsonSchema;
use serde::Deserialize;

/// Element criteria shared by every tool that searches for elements, with an optional scope.
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct QueryArguments {
    #[serde(flatten)]
    pub criteria: CriteriaArguments,
    /// Only match inside an element matching these criteria, e.g.
    /// `{ "role": "window", "title": "Invoice" }` or the chat pane of a messaging app.
    pub within: Option<CriteriaArguments>,
}

/// The criteria one element must match.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CriteriaArguments {
    /// Role to match, e.g. "button" or "AXButton".
    pub role: Option<String>,
    /// Text the title or description must contain (case-insensitive).
    pub title: Option<String>,
    /// Text the value must contain (case-insensitive).
    pub value: Option<String>,
    /// Text the placeholder (hint in an empty field) must contain (case-insensitive).
    pub placeholder: Option<String>,
    /// The element's accessibility identifier, matched exactly (like a test ID).
    pub identifier: Option<String>,
    /// Match title, value and placeholder exactly (ignoring case) instead of as substrings.
    #[serde(default)]
    pub exact: bool,
}

impl QueryArguments {
    /// Whether the element itself was described (a scope alone describes nothing).
    pub fn has_criteria(&self) -> bool {
        self.criteria.has_criteria()
    }

    pub fn to_query(&self) -> DeskwrightResult<ElementQuery> {
        let query = self.criteria.to_query()?;
        match &self.within {
            Some(scope) => Ok(query.within(scope.to_query()?)),
            None => Ok(query),
        }
    }
}

impl CriteriaArguments {
    pub fn has_criteria(&self) -> bool {
        self.role.is_some()
            || self.title.is_some()
            || self.value.is_some()
            || self.placeholder.is_some()
            || self.identifier.is_some()
    }

    pub fn to_query(&self) -> DeskwrightResult<ElementQuery> {
        ElementQuery::from_criteria(QueryCriteria {
            role: self.role.as_deref(),
            title: self.title.as_deref(),
            value: self.value.as_deref(),
            placeholder: self.placeholder.as_deref(),
            identifier: self.identifier.as_deref(),
            is_exact: self.exact,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_identifier(identifier: &str) -> CriteriaArguments {
        CriteriaArguments {
            identifier: Some(identifier.to_owned()),
            ..CriteriaArguments::default()
        }
    }

    #[test]
    fn reports_whether_any_criterion_was_given() {
        let empty = QueryArguments::default();
        let only_scope = QueryArguments {
            within: Some(by_identifier("compose")),
            ..QueryArguments::default()
        };
        let with_criterion = QueryArguments {
            criteria: by_identifier("send"),
            within: None,
        };

        assert!(!empty.has_criteria());
        assert!(!only_scope.has_criteria());
        assert!(with_criterion.has_criteria());
        assert!(empty.to_query().is_err());
        assert!(with_criterion.to_query().is_ok());
    }

    #[test]
    fn scopes_the_query_and_validates_the_scope() {
        let scoped = QueryArguments {
            criteria: by_identifier("send"),
            within: Some(by_identifier("compose")),
        };
        let empty_scope = QueryArguments {
            criteria: by_identifier("send"),
            within: Some(CriteriaArguments::default()),
        };

        assert_eq!(
            scoped.to_query().unwrap().to_string(),
            "identifier 'send' within (identifier 'compose')"
        );
        assert!(empty_scope.to_query().is_err());
    }
}
