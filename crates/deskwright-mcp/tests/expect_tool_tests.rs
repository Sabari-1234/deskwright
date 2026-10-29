mod support;

use serde_json::json;
use support::{Fixture, text_of};

#[tokio::test]
async fn passes_when_the_condition_holds() {
    let fixture = Fixture::new();

    let visible = fixture
        .call(
            "desk_expect",
            json!({ "role": "button", "title": "Send", "condition": "visible" }),
        )
        .await;
    let enabled = fixture
        .call(
            "desk_expect",
            json!({ "role": "button", "condition": "enabled" }),
        )
        .await;
    let count = fixture
        .call(
            "desk_expect",
            json!({ "role": "textfield", "condition": "count", "count": 1 }),
        )
        .await;

    assert_eq!(
        text_of(&visible),
        "Passed: role 'button', title 'send' to be visible (e2)."
    );
    assert!(!enabled.is_error);
    assert!(text_of(&count).starts_with("Passed: role 'textfield' to match 1 elements (e"));
}

#[tokio::test]
async fn checks_text_exactly_or_by_substring() {
    let fixture = Fixture::new();
    fixture
        .call(
            "desk_type",
            json!({ "role": "textfield", "title": "Message", "text": "hi there" }),
        )
        .await;

    let contains = fixture
        .call(
            "desk_expect",
            json!({ "role": "textfield", "condition": "containsText", "text": "THERE" }),
        )
        .await;
    let exact = fixture
        .call(
            "desk_expect",
            json!({ "role": "textfield", "condition": "hasText", "text": "hi", "timeoutSeconds": 0 }),
        )
        .await;

    assert!(!contains.is_error);
    assert!(exact.is_error);
    assert!(text_of(&exact).ends_with(" shows \"hi there\""));
}

#[tokio::test]
async fn fails_with_what_it_saw() {
    let fixture = Fixture::new();

    let output = fixture
        .call(
            "desk_expect",
            json!({ "title": "Send", "condition": "disabled", "timeoutSeconds": 0 }),
        )
        .await;

    assert!(output.is_error);
    assert_eq!(
        text_of(&output),
        "expected title 'send' to be disabled within 0 ms, but e2 is enabled"
    );
}

#[tokio::test]
async fn rejects_refs_and_missing_condition_arguments() {
    let fixture = Fixture::new();

    let with_ref = fixture
        .call(
            "desk_expect",
            json!({ "ref": "e2", "condition": "visible" }),
        )
        .await;
    let no_text = fixture
        .call(
            "desk_expect",
            json!({ "role": "button", "condition": "hasText" }),
        )
        .await;
    let no_count = fixture
        .call(
            "desk_expect",
            json!({ "role": "button", "condition": "count" }),
        )
        .await;

    assert!(text_of(&with_ref).contains("not a ref"));
    assert!(text_of(&no_text).contains("need text"));
    assert!(text_of(&no_count).contains("needs count"));
}

#[tokio::test]
async fn wait_for_hidden_returns_once_nothing_is_visible() {
    let fixture = Fixture::new();

    let gone = fixture
        .call(
            "desk_wait_for",
            json!({ "role": "progressindicator", "state": "hidden" }),
        )
        .await;
    let still_there = fixture
        .call(
            "desk_wait_for",
            json!({ "title": "Send", "state": "hidden", "timeoutSeconds": 0 }),
        )
        .await;

    assert_eq!(
        text_of(&gone),
        "No element matching role 'progressindicator' is visible."
    );
    assert!(still_there.is_error);
    assert!(text_of(&still_there).ends_with(" is still visible"));
}

#[tokio::test]
async fn suggests_a_locator_for_a_snapshot_ref() {
    let fixture = Fixture::new();
    fixture.call("desk_snapshot", json!({})).await;

    let suggested = fixture
        .call("desk_locator", json!({ "ref": "e2", "app": "Notes" }))
        .await;
    let malformed = fixture
        .call("desk_locator", json!({ "ref": "button" }))
        .await;

    assert_eq!(
        text_of(&suggested),
        r#"{"app":"Notes","exact":true,"role":"button","title":"Send"}"#
    );
    assert!(malformed.is_error);
}
