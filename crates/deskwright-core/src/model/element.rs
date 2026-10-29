use std::fmt;

use serde::{Serialize, Serializer};

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::geometry::Bounds;

/// A short identifier for a UI element, such as `e12`. Numbers keep increasing across
/// snapshots, so a ref from an older snapshot never names an element of a newer one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ElementRef(usize);

impl ElementRef {
    const PREFIX: char = 'e';

    pub fn from_index(index: usize) -> Self {
        Self(index)
    }

    pub fn parse(raw: &str) -> DeskwrightResult<Self> {
        let trimmed = raw.trim();
        trimmed
            .strip_prefix(Self::PREFIX)
            .filter(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
            .and_then(|digits| digits.parse().ok())
            .map(Self)
            .ok_or_else(|| {
                DeskwrightError::InvalidInput(format!(
                    "'{raw}' is not an element ref; refs look like 'e12'"
                ))
            })
    }

    pub fn index(self) -> usize {
        self.0
    }
}

impl fmt::Display for ElementRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", Self::PREFIX, self.0)
    }
}

impl Serialize for ElementRef {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// The readable state of one UI element.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementAttributes {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Hint text shown in an empty field, such as "Search".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// A developer-assigned identifier that doesn't change with language or content, the
    /// macOS equivalent of a test ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    pub is_enabled: bool,
    pub is_focused: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Bounds>,
}

/// Accessibility values that mean "checked" for checkboxes, radio buttons and switches.
const CHECKED_VALUES: [&str; 2] = ["1", "true"];

impl ElementAttributes {
    /// Whether a checkbox, radio button or switch is on.
    pub fn is_checked(&self) -> bool {
        self.value
            .as_deref()
            .is_some_and(|value| CHECKED_VALUES.contains(&value.to_ascii_lowercase().as_str()))
    }

    /// Whether the element takes up visible space on screen.
    pub fn is_visible(&self) -> bool {
        self.bounds.is_some_and(|bounds| !bounds.is_empty())
    }
}

/// One element with its ref, without its children.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ElementSummary {
    #[serde(rename = "ref")]
    pub element_ref: ElementRef,
    #[serde(flatten)]
    pub attributes: ElementAttributes,
}

/// A node in a UI element tree (Composite: leaves and containers share one type).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ElementNode {
    #[serde(flatten)]
    pub summary: ElementSummary,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ElementNode>,
}

impl ElementNode {
    /// Iterates this node and all of its descendants, depth-first.
    pub fn depth_first(&self) -> DepthFirstIter<'_> {
        DepthFirstIter {
            pending: vec![self],
        }
    }
}

pub struct DepthFirstIter<'tree> {
    pending: Vec<&'tree ElementNode>,
}

impl<'tree> Iterator for DepthFirstIter<'tree> {
    type Item = &'tree ElementNode;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.pending.pop()?;
        self.pending.extend(node.children.iter().rev());
        Some(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_refs_and_rejects_others() {
        assert_eq!(
            ElementRef::parse(" e12 ").unwrap(),
            ElementRef::from_index(12)
        );
        assert_eq!(ElementRef::from_index(7).to_string(), "e7");
        assert!(ElementRef::parse("e").is_err());
        assert!(ElementRef::parse("x12").is_err());
        assert!(ElementRef::parse("e1a").is_err());
        assert!(ElementRef::parse("e99999999999999999999999").is_err());
    }

    #[test]
    fn depth_first_visits_parents_before_children_in_order() {
        let leaf = |index| ElementNode {
            summary: ElementSummary {
                element_ref: ElementRef::from_index(index),
                attributes: ElementAttributes::default(),
            },
            children: Vec::new(),
        };
        let mut root = leaf(1);
        let mut middle = leaf(2);
        middle.children.push(leaf(3));
        root.children.extend([middle, leaf(4)]);

        let visited: Vec<_> = root
            .depth_first()
            .map(|node| node.summary.element_ref.to_string())
            .collect();

        assert_eq!(visited, ["e1", "e2", "e3", "e4"]);
    }
}
