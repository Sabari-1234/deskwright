use std::time::Duration;

use crate::model::app::AppSelector;
use crate::model::element::ElementRef;
use crate::model::element_query::ElementQuery;

/// Which of the matching elements a locator picks, in tree (reading) order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchPosition {
    /// 0-based: `Nth(0)` is the first match.
    Nth(usize),
    Last,
}

/// Finds an element by what it is rather than by a ref, like a Playwright locator: the match
/// at `position` among elements matching `query` in `app`, waiting up to `timeout` for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locator {
    pub app: AppSelector,
    pub query: ElementQuery,
    pub position: MatchPosition,
    pub timeout: Duration,
}

/// Which element an action applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementTarget {
    /// A ref from the latest snapshot or search.
    Ref(ElementRef),
    /// An element found (and waited for) at the moment of the action.
    Locator(Locator),
}
