use serde::Serialize;

use crate::model::app::AppInfo;
use crate::model::element::ElementNode;

/// The UI element tree of one app at one moment, with refs valid until the next snapshot.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub app: AppInfo,
    pub elements: Vec<ElementNode>,
    pub element_count: usize,
    /// Elements that disappeared or could not be read while the tree was walked.
    pub unreadable_element_count: usize,
    /// True when the element limit stopped the walk before the whole tree was read.
    pub is_truncated: bool,
}
