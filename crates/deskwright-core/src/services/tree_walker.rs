use std::collections::HashMap;

use crate::model::element::{ElementAttributes, ElementNode, ElementRef, ElementSummary};
use crate::model::element_query::ElementQuery;
use crate::model::native_element::NativeElement;
use crate::model::tree_depth::TreeDepth;
use crate::ports::ElementTreeReader;
use crate::ports::element_tree_reader::ElementContents;

/// The result of walking an app's element tree: the nodes plus the ref → element mapping.
pub(crate) struct WalkedTree {
    pub roots: Vec<ElementNode>,
    pub elements_by_ref: HashMap<ElementRef, NativeElement>,
    pub unreadable_element_count: usize,
    pub is_truncated: bool,
}

/// Stop walking once this many elements match `query`, because the rest can't change the
/// answer to a search.
pub(crate) struct MatchGoal {
    pub query: ElementQuery,
    pub needed_matches: usize,
}

/// Walks an element tree depth-first, assigning consecutive refs in visiting order.
pub(crate) struct TreeWalker<'reader> {
    tree: &'reader dyn ElementTreeReader,
    goal: Option<MatchGoal>,
    match_count: usize,
    first_index: usize,
    max_depth: usize,
    max_elements: usize,
    elements_by_ref: HashMap<ElementRef, NativeElement>,
    unreadable_element_count: usize,
    is_truncated: bool,
}

impl<'reader> TreeWalker<'reader> {
    /// Numbers elements from `first_index` in visiting order. With a `goal`, stops as soon
    /// as enough elements match.
    pub fn walk(
        tree: &'reader dyn ElementTreeReader,
        roots: &[NativeElement],
        depth: TreeDepth,
        max_elements: usize,
        first_index: usize,
        goal: Option<MatchGoal>,
    ) -> WalkedTree {
        let mut walker = Self {
            goal,
            match_count: 0,
            first_index,
            tree,
            max_depth: depth.levels(),
            max_elements,
            elements_by_ref: HashMap::new(),
            unreadable_element_count: 0,
            is_truncated: false,
        };
        let root_nodes = roots
            .iter()
            .filter_map(|root| walker.visit(root, 0, false))
            .collect();
        WalkedTree {
            roots: root_nodes,
            elements_by_ref: walker.elements_by_ref,
            unreadable_element_count: walker.unreadable_element_count,
            is_truncated: walker.is_truncated,
        }
    }

    /// `is_inside_scope` says whether an ancestor matched the goal query's scope.
    fn visit(
        &mut self,
        element: &NativeElement,
        level: usize,
        is_inside_scope: bool,
    ) -> Option<ElementNode> {
        if self.should_stop() {
            return None;
        }
        let reads_children = level < self.max_depth;
        let contents = if reads_children {
            self.tree.read_with_children(element)
        } else {
            self.tree
                .attributes(element)
                .map(|attributes| ElementContents {
                    attributes,
                    children: Vec::new(),
                })
        };
        let Ok(ElementContents {
            attributes,
            children,
        }) = contents
        else {
            // Elements routinely vanish mid-walk (closed popovers, re-rendered lists);
            // the snapshot reports how many were skipped instead of failing outright.
            self.unreadable_element_count += 1;
            return None;
        };
        let element_ref = ElementRef::from_index(self.first_index + self.elements_by_ref.len());
        self.elements_by_ref.insert(element_ref, element.clone());
        self.count_if_match(&attributes, is_inside_scope);
        let are_children_inside = is_inside_scope
            || self
                .goal
                .as_ref()
                .is_some_and(|goal| goal.query.opens_scope(&attributes));
        let child_nodes = children
            .iter()
            .filter_map(|child| self.visit(child, level + 1, are_children_inside))
            .collect();
        Some(ElementNode {
            summary: ElementSummary {
                element_ref,
                attributes,
            },
            children: child_nodes,
        })
    }

    fn should_stop(&mut self) -> bool {
        if self.elements_by_ref.len() >= self.max_elements {
            self.is_truncated = true;
            return true;
        }
        self.goal
            .as_ref()
            .is_some_and(|goal| self.match_count >= goal.needed_matches)
    }

    fn count_if_match(&mut self, attributes: &ElementAttributes, is_inside_scope: bool) {
        if self
            .goal
            .as_ref()
            .is_some_and(|goal| goal.query.matches_in_scope(attributes, is_inside_scope))
        {
            self.match_count += 1;
        }
    }
}
