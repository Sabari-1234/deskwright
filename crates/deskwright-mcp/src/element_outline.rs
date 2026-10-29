//! Compact text rendering of elements: one line per element, children indented. Much
//! shorter than JSON, so AI clients read UI state faster and with fewer tokens.

use std::fmt::Write;

use deskwright_core::model::app::AppInfo;
use deskwright_core::model::element::{ElementAttributes, ElementNode, ElementSummary};
use deskwright_core::model::snapshot::AppSnapshot;
use deskwright_core::services::element_search_service::SearchResults;

const ACCESSIBILITY_ROLE_PREFIX: &str = "AX";
const INDENT: &str = "  ";
/// Untitled elements with these roles only arrange their children, so they get no line of
/// their own; their children are shown one level up instead. Their refs stay valid.
const LAYOUT_ONLY_ROLES: [&str; 6] = [
    "Group",
    "ScrollArea",
    "SplitGroup",
    "LayoutArea",
    "Unknown",
    "Splitter",
];

pub fn render_snapshot(snapshot: &AppSnapshot) -> String {
    let mut outline = app_header(&snapshot.app, snapshot.element_count);
    if snapshot.unreadable_element_count > 0 {
        let _ = write!(
            outline,
            "; {} could not be read",
            snapshot.unreadable_element_count
        );
    }
    if snapshot.is_truncated {
        outline.push_str("; truncated at the element limit, query a smaller part");
    }
    outline.push('\n');
    for root in &snapshot.elements {
        render_node(root, 0, &mut outline);
    }
    outline
}

/// Renders search matches, one line each, noting when more matches exist.
pub fn render_search_results(results: &SearchResults) -> String {
    if results.matches.is_empty() {
        return "No matching elements.".to_owned();
    }
    let mut outline = String::new();
    for summary in &results.matches {
        outline.push_str(&summary_line(summary));
        outline.push('\n');
    }
    if results.has_more {
        outline.push_str("… more matches exist; narrow the query or raise the limit\n");
    }
    outline
}

fn app_header(app: &AppInfo, element_count: usize) -> String {
    let frontmost = if app.is_active { ", frontmost" } else { "" };
    let bundle_id = app
        .bundle_id
        .as_deref()
        .map(|bundle_id| format!("{bundle_id}, "))
        .unwrap_or_default();
    format!(
        "{} ({bundle_id}pid {}{frontmost}): {element_count} elements",
        app.name, app.process_id.0
    )
}

fn render_node(node: &ElementNode, depth: usize, outline: &mut String) {
    let child_depth = if is_layout_only(&node.summary.attributes) {
        depth
    } else {
        outline.push_str(&INDENT.repeat(depth));
        outline.push_str(&summary_line(&node.summary));
        outline.push('\n');
        depth + 1
    };
    for child in &node.children {
        render_node(child, child_depth, outline);
    }
}

fn summary_line(summary: &ElementSummary) -> String {
    let attributes = &summary.attributes;
    let mut line = format!("[{}] {}", summary.element_ref, short_role(&attributes.role));
    for (label, text) in [
        ("", attributes.title.as_deref()),
        ("value=", attributes.value.as_deref()),
        ("desc=", attributes.description.as_deref()),
        ("placeholder=", attributes.placeholder.as_deref()),
        ("id=", attributes.identifier.as_deref()),
    ] {
        if let Some(text) = text {
            let _ = write!(line, " {label}{}", quoted_single_line(text));
        }
    }
    if !attributes.is_enabled {
        line.push_str(" (disabled)");
    }
    if attributes.is_focused {
        line.push_str(" (focused)");
    }
    line
}

fn short_role(role: &str) -> &str {
    role.strip_prefix(ACCESSIBILITY_ROLE_PREFIX).unwrap_or(role)
}

fn is_layout_only(attributes: &ElementAttributes) -> bool {
    let has_text = attributes.title.is_some()
        || attributes.value.is_some()
        || attributes.description.is_some()
        || attributes.placeholder.is_some()
        || attributes.identifier.is_some();
    !has_text && LAYOUT_ONLY_ROLES.contains(&short_role(&attributes.role))
}

/// Quotes text on one line, so multi-line values can't break the outline's structure.
fn quoted_single_line(text: &str) -> String {
    format!("{:?}", text)
}

#[cfg(test)]
mod tests {
    use deskwright_core::model::app::ProcessId;
    use deskwright_core::model::element::ElementRef;

    use super::*;

    fn summary(index: usize, role: &str, title: Option<&str>) -> ElementSummary {
        ElementSummary {
            element_ref: ElementRef::from_index(index),
            attributes: ElementAttributes {
                role: role.to_owned(),
                title: title.map(str::to_owned),
                is_enabled: true,
                ..ElementAttributes::default()
            },
        }
    }

    fn node(summary: ElementSummary, children: Vec<ElementNode>) -> ElementNode {
        ElementNode { summary, children }
    }

    fn snapshot(elements: Vec<ElementNode>) -> AppSnapshot {
        AppSnapshot {
            app: AppInfo {
                name: "Notes".to_owned(),
                bundle_id: Some("com.apple.Notes".to_owned()),
                process_id: ProcessId(42),
                is_active: true,
            },
            elements,
            element_count: 4,
            unreadable_element_count: 0,
            is_truncated: false,
        }
    }

    #[test]
    fn renders_an_indented_outline_and_skips_untitled_layout_groups() {
        let tree = node(
            summary(1, "AXWindow", Some("Main")),
            vec![node(
                summary(2, "AXGroup", None),
                vec![
                    node(summary(3, "AXButton", Some("Send")), Vec::new()),
                    node(summary(4, "AXTextField", None), Vec::new()),
                ],
            )],
        );

        let outline = render_snapshot(&snapshot(vec![tree]));

        assert_eq!(
            outline,
            "Notes (com.apple.Notes, pid 42, frontmost): 4 elements\n\
             [e1] Window \"Main\"\n  [e3] Button \"Send\"\n  [e4] TextField\n"
        );
    }

    #[test]
    fn keeps_groups_that_carry_text() {
        let labelled_group = node(summary(1, "AXGroup", Some("Toolbar")), Vec::new());

        assert!(
            render_snapshot(&snapshot(vec![labelled_group])).contains("[e1] Group \"Toolbar\"")
        );
    }

    #[test]
    fn shows_values_descriptions_and_states_on_one_line() {
        let mut field = summary(7, "AXTextArea", None);
        field.attributes.value = Some("line one\nline two".to_owned());
        field.attributes.description = Some("editor".to_owned());
        field.attributes.placeholder = Some("Type here".to_owned());
        field.attributes.identifier = Some("editor-area".to_owned());
        field.attributes.is_enabled = false;
        field.attributes.is_focused = true;

        let outline = render_search_results(&SearchResults {
            matches: vec![field],
            has_more: false,
        });

        assert_eq!(
            outline,
            "[e7] TextArea value=\"line one\\nline two\" desc=\"editor\" placeholder=\"Type here\" id=\"editor-area\" (disabled) (focused)\n"
        );
    }

    #[test]
    fn reports_truncated_and_unreadable_snapshots_in_the_header() {
        let mut partial = snapshot(Vec::new());
        partial.unreadable_element_count = 2;
        partial.is_truncated = true;

        let header = render_snapshot(&partial);

        assert!(header.contains("2 could not be read"));
        assert!(header.contains("truncated at the element limit"));
    }

    #[test]
    fn notes_when_more_matches_exist() {
        let results = SearchResults {
            matches: (1..=2)
                .map(|index| summary(index, "AXButton", None))
                .collect(),
            has_more: true,
        };

        let outline = render_search_results(&results);

        assert_eq!(outline.lines().count(), 3);
        assert!(outline.ends_with("… more matches exist; narrow the query or raise the limit\n"));
    }

    #[test]
    fn says_so_when_nothing_matches() {
        let empty = SearchResults {
            matches: Vec::new(),
            has_more: false,
        };

        assert_eq!(render_search_results(&empty), "No matching elements.");
    }
}
