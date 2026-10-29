mod support;

use deskwright_mcp::tool_registry::{DuplicateToolName, ToolRegistry};
use deskwright_mcp::tools::BatchTool;
use serde_json::json;
use support::{Fixture, text_of};

#[test]
fn rejects_two_tools_with_the_same_name() {
    let mut registry = ToolRegistry::new();
    registry.register(BatchTool).unwrap();

    let duplicate = registry.register(BatchTool).map(|_| ()).unwrap_err();

    assert_eq!(duplicate, DuplicateToolName("desk_batch"));
    assert_eq!(
        duplicate.to_string(),
        "a tool named 'desk_batch' is already registered"
    );
}

#[test]
fn looks_tools_up_by_name() {
    let fixture = Fixture::new();

    assert!(fixture.registry.contains("desk_click"));
    assert!(!fixture.registry.contains("desk_teleport"));
    assert_eq!(
        fixture.registry.spec("desk_read").map(|spec| spec.name),
        Some("desk_read")
    );
    assert!(fixture.registry.spec("desk_teleport").is_none());
}

#[test]
fn lists_specs_sorted_by_name() {
    let fixture = Fixture::new();

    let names: Vec<_> = fixture
        .registry
        .specs()
        .into_iter()
        .map(|spec| spec.name)
        .collect();

    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
    assert_eq!(names.len(), 24);
}

#[test]
fn every_tool_advertises_an_object_input_schema() {
    let fixture = Fixture::new();

    for spec in fixture.registry.specs() {
        assert_eq!(
            spec.input_schema.get("type"),
            Some(&json!("object")),
            "{}",
            spec.name
        );
    }
}

#[test]
fn only_reading_tools_are_marked_read_only() {
    let fixture = Fixture::new();

    let read_only: Vec<_> = fixture
        .registry
        .specs()
        .into_iter()
        .filter(|spec| spec.is_read_only)
        .map(|spec| spec.name)
        .collect();

    assert_eq!(
        read_only,
        [
            "desk_expect",
            "desk_find",
            "desk_list_apps",
            "desk_locator",
            "desk_read",
            "desk_read_clipboard",
            "desk_screenshot",
            "desk_snapshot",
            "desk_wait_for"
        ]
    );
}

#[tokio::test]
async fn reports_unknown_tools_and_invalid_arguments_as_errors() {
    let fixture = Fixture::new();

    let unknown = fixture.call("desk_teleport", json!({})).await;
    let wrongly_typed = fixture.call("desk_click", json!({ "nth": "first" })).await;
    let missing_target = fixture.call("desk_click", json!({})).await;

    assert!(unknown.is_error);
    assert!(text_of(&unknown).contains("unknown tool"));
    assert!(wrongly_typed.is_error);
    assert!(text_of(&wrongly_typed).contains("invalid arguments"));
    assert!(missing_target.is_error);
    assert!(text_of(&missing_target).contains("give a ref or a locator"));
}
