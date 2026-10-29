mod support;

use deskwright_core::testing::RecordedAction;
use deskwright_mcp::tool::OutputBlock;
use serde_json::{Value, json};
use support::{Fixture, MESSAGE_FIELD_REF, SEND_BUTTON_REF, WINDOW_REF, text_of};

fn parse_json(text: &str) -> Value {
    serde_json::from_str(text).expect("tool output is JSON")
}

#[tokio::test]
async fn snapshot_returns_a_compact_outline_by_default() {
    let fixture = Fixture::new();

    let snapshot = fixture
        .call("desk_snapshot", json!({ "app": "notes" }))
        .await;

    assert_eq!(
        text_of(&snapshot),
        "Notes (com.apple.Notes, pid 100, frontmost): 3 elements\n\
         [e1] Window \"Main\"\n  [e2] Button \"Send\"\n  [e3] TextField \"Message\" value=\"\"\n"
    );
}

#[tokio::test]
async fn snapshot_returns_the_tree_with_refs_as_json_when_asked() {
    let fixture = Fixture::new();

    let snapshot = fixture
        .call("desk_snapshot", json!({ "app": "notes", "format": "json" }))
        .await;

    assert!(!snapshot.is_error, "{}", text_of(&snapshot));
    let tree = parse_json(&text_of(&snapshot));
    assert_eq!(tree["app"]["name"], "Notes");
    assert_eq!(tree["elementCount"], 3);
    assert_eq!(tree["elements"][0]["ref"], WINDOW_REF);
    assert_eq!(tree["elements"][0]["children"][0]["title"], "Send");
}

#[tokio::test]
async fn snapshot_rejects_an_out_of_range_depth() {
    let fixture = Fixture::new();

    let snapshot = fixture
        .call("desk_snapshot", json!({ "maxDepth": 0 }))
        .await;

    assert!(snapshot.is_error);
    assert!(text_of(&snapshot).contains("tree depth"));
}

#[tokio::test]
async fn click_uses_refs_from_the_latest_snapshot() {
    let fixture = Fixture::new();
    fixture.call("desk_snapshot", json!({})).await;

    let click = fixture
        .call("desk_click", json!({ "ref": SEND_BUTTON_REF }))
        .await;

    assert_eq!(text_of(&click), "Clicked e2 using accessibility press.");
    assert!(matches!(
        fixture.desktop.recorded_actions().as_slice(),
        [RecordedAction::Press(_)]
    ));
}

#[tokio::test]
async fn click_rejects_malformed_and_unknown_refs() {
    let fixture = Fixture::new();

    let malformed = fixture.call("desk_click", json!({ "ref": "button" })).await;
    let unknown = fixture.call("desk_click", json!({ "ref": "e42" })).await;

    assert!(text_of(&malformed).contains("is not an element ref"));
    assert!(text_of(&unknown).contains("take a new snapshot"));
}

#[tokio::test]
async fn typing_confirms_without_echoing_the_text() {
    let fixture = Fixture::new();
    fixture.call("desk_snapshot", json!({})).await;

    let typed = fixture
        .call(
            "desk_type",
            json!({ "ref": MESSAGE_FIELD_REF, "text": "hunter2" }),
        )
        .await;

    assert!(!typed.is_error, "{}", text_of(&typed));
    assert!(!text_of(&typed).contains("hunter2"));
}

#[tokio::test]
async fn typing_rejects_malformed_refs() {
    let fixture = Fixture::new();

    let typed = fixture
        .call("desk_type", json!({ "ref": "field", "text": "x" }))
        .await;

    assert!(typed.is_error);
}

#[tokio::test]
async fn read_returns_the_live_value_of_an_element() {
    let fixture = Fixture::new();
    fixture.call("desk_snapshot", json!({})).await;
    fixture
        .call(
            "desk_type",
            json!({ "ref": MESSAGE_FIELD_REF, "text": "hello" }),
        )
        .await;

    let read = fixture
        .call("desk_read", json!({ "ref": MESSAGE_FIELD_REF }))
        .await;

    let summary = parse_json(&text_of(&read));
    assert_eq!(summary["ref"], MESSAGE_FIELD_REF);
    assert_eq!(summary["value"], "hello");
}

#[tokio::test]
async fn read_rejects_malformed_refs() {
    let fixture = Fixture::new();

    let read = fixture.call("desk_read", json!({ "ref": "3" })).await;

    assert!(read.is_error);
}

#[tokio::test]
async fn find_returns_one_line_per_match_by_default() {
    let fixture = Fixture::new();

    let found = fixture
        .call("desk_find", json!({ "role": "textfield" }))
        .await;

    assert_eq!(text_of(&found), "[e3] TextField \"Message\" value=\"\"\n");
}

#[tokio::test]
async fn find_limits_matches_and_rejects_invalid_limits() {
    let fixture = Fixture::new();

    let limited = fixture
        .call("desk_find", json!({ "title": "e", "limit": 1 }))
        .await;
    let invalid = fixture
        .call("desk_find", json!({ "title": "e", "limit": 0 }))
        .await;

    assert!(text_of(&limited).contains("more matches exist"));
    assert!(text_of(&invalid).contains("limit must be between 1 and 200"));
}

#[tokio::test]
async fn wait_for_returns_matches_that_already_exist() {
    let fixture = Fixture::new();

    let found = fixture
        .call(
            "desk_wait_for",
            json!({ "role": "button", "title": "Send" }),
        )
        .await;

    assert_eq!(text_of(&found), "[e2] Button \"Send\"\n");
}

#[tokio::test]
async fn wait_for_times_out_and_rejects_bad_timeouts() {
    let fixture = Fixture::new();

    let missing = fixture
        .call(
            "desk_wait_for",
            json!({ "title": "Nope", "timeoutSeconds": 0 }),
        )
        .await;
    let negative = fixture
        .call(
            "desk_wait_for",
            json!({ "title": "Send", "timeoutSeconds": -1 }),
        )
        .await;

    assert!(text_of(&missing).contains("no element matching title 'nope' appeared"));
    assert!(text_of(&negative).contains("non-negative number"));
}

#[tokio::test]
async fn find_returns_only_matching_elements_as_json_when_asked() {
    let fixture = Fixture::new();

    let found = fixture
        .call(
            "desk_find",
            json!({ "role": "button", "app": "Notes", "format": "json" }),
        )
        .await;

    let matches = parse_json(&text_of(&found));
    assert_eq!(matches.as_array().map(Vec::len), Some(1));
    assert_eq!(matches[0]["title"], "Send");
}

#[tokio::test]
async fn find_requires_at_least_one_criterion() {
    let fixture = Fixture::new();

    let found = fixture.call("desk_find", json!({})).await;

    assert!(found.is_error);
    assert!(
        text_of(&found).contains("at least one of role, title, value, placeholder or identifier")
    );
}

#[tokio::test]
async fn screenshot_of_an_element_or_the_display_returns_a_png_block() {
    let fixture = Fixture::new();
    fixture.call("desk_snapshot", json!({})).await;

    let element_shot = fixture
        .call("desk_screenshot", json!({ "ref": SEND_BUTTON_REF }))
        .await;
    let display_shot = fixture.call("desk_screenshot", json!({})).await;

    for screenshot in [element_shot, display_shot] {
        assert!(matches!(
            screenshot.blocks.as_slice(),
            [OutputBlock::PngImage { .. }]
        ));
    }
}

#[tokio::test]
async fn screenshot_rejects_malformed_refs() {
    let fixture = Fixture::new();

    let screenshot = fixture.call("desk_screenshot", json!({ "ref": "?" })).await;

    assert!(screenshot.is_error);
}

#[tokio::test]
async fn refs_from_an_older_snapshot_are_refused_instead_of_remapped() {
    let fixture = Fixture::new();
    fixture.call("desk_snapshot", json!({})).await;
    fixture.call("desk_find", json!({ "role": "window" })).await;

    let stale_click = fixture
        .call("desk_click", json!({ "ref": SEND_BUTTON_REF }))
        .await;

    assert!(stale_click.is_error);
    assert!(text_of(&stale_click).contains("take a new snapshot"));
    assert!(fixture.desktop.recorded_actions().is_empty());
}

#[tokio::test]
async fn click_finds_the_element_by_locator_without_a_snapshot() {
    let fixture = Fixture::new();

    let click = fixture
        .call(
            "desk_click",
            json!({ "app": "Notes", "role": "button", "title": "send" }),
        )
        .await;

    assert!(!click.is_error, "{}", text_of(&click));
    assert!(matches!(
        fixture.desktop.recorded_actions().as_slice(),
        [RecordedAction::Press(_)]
    ));
}

#[tokio::test]
async fn type_and_read_by_locator() {
    let fixture = Fixture::new();

    let typed = fixture
        .call(
            "desk_type",
            json!({ "role": "textfield", "title": "Message", "text": "hello" }),
        )
        .await;
    let read = fixture
        .call(
            "desk_read",
            json!({ "role": "textfield", "title": "Message" }),
        )
        .await;

    assert!(!typed.is_error, "{}", text_of(&typed));
    assert_eq!(parse_json(&text_of(&read))["value"], "hello");
}

#[tokio::test]
async fn locators_wait_then_report_missing_elements() {
    let fixture = Fixture::new();

    let click = fixture
        .call(
            "desk_click",
            json!({ "title": "Nope", "timeoutSeconds": 0 }),
        )
        .await;
    let both = fixture
        .call("desk_click", json!({ "ref": "e2", "role": "button" }))
        .await;

    assert!(text_of(&click).contains("no element matching title 'nope' appeared"));
    assert!(text_of(&both).contains("not both"));
    assert!(fixture.desktop.recorded_actions().is_empty());
}
