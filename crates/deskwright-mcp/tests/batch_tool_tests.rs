mod support;

use deskwright_core::testing::RecordedAction;
use serde_json::json;
use support::{Fixture, SEND_BUTTON_REF, text_of};

#[tokio::test]
async fn runs_actions_in_order_and_stops_at_the_first_failure() {
    let fixture = Fixture::new();

    let batch = fixture
        .call(
            "desk_batch",
            json!({ "actions": [
                { "tool": "desk_snapshot" },
                { "tool": "desk_click", "params": { "ref": "e99" } },
                { "tool": "desk_click", "params": { "ref": SEND_BUTTON_REF } },
            ]}),
        )
        .await;

    assert!(batch.is_error);
    let summary = text_of(&batch);
    assert!(summary.contains("[0] desk_snapshot ok"));
    assert!(summary.contains("[1] desk_click failed"));
    assert!(!summary.contains("[2]"));
    assert!(fixture.desktop.recorded_actions().is_empty());
}

#[tokio::test]
async fn continues_after_failures_when_asked() {
    let fixture = Fixture::new();

    let batch = fixture
        .call(
            "desk_batch",
            json!({ "continueOnError": true, "actions": [
                { "tool": "desk_snapshot" },
                { "tool": "desk_click", "params": { "ref": "e99" } },
                { "tool": "desk_click", "params": { "ref": SEND_BUTTON_REF } },
            ]}),
        )
        .await;

    assert!(batch.is_error);
    assert!(text_of(&batch).contains("[2] desk_click ok"));
    assert!(matches!(
        fixture.desktop.recorded_actions().as_slice(),
        [RecordedAction::Press(_)]
    ));
}

#[tokio::test]
async fn succeeds_when_every_action_succeeds() {
    let fixture = Fixture::new();

    let batch = fixture
        .call(
            "desk_batch",
            json!({ "actions": [
                { "tool": "desk_snapshot" },
                { "tool": "desk_click", "params": { "ref": SEND_BUTTON_REF } },
            ]}),
        )
        .await;

    assert!(!batch.is_error, "{}", text_of(&batch));
}

#[tokio::test]
async fn rejects_empty_oversized_and_nested_batches() {
    let fixture = Fixture::new();
    let too_many: Vec<_> = (0..51)
        .map(|_| json!({ "tool": "desk_list_apps" }))
        .collect();

    let empty = fixture.call("desk_batch", json!({ "actions": [] })).await;
    let oversized = fixture
        .call("desk_batch", json!({ "actions": too_many }))
        .await;
    let nested = fixture
        .call(
            "desk_batch",
            json!({ "actions": [{ "tool": "desk_batch", "params": { "actions": [] } }] }),
        )
        .await;

    assert!(text_of(&empty).contains("at least one action"));
    assert!(text_of(&oversized).contains("at most 50 actions, got 51"));
    assert!(text_of(&nested).contains("cannot contain desk_batch"));
}

#[tokio::test]
async fn runs_a_locator_flow_without_any_snapshot() {
    let fixture = Fixture::new();

    let flow = fixture
        .call(
            "desk_batch",
            json!({ "actions": [
                { "tool": "desk_launch_app", "params": { "app": "Notes" } },
                { "tool": "desk_type", "params": { "app": "Notes", "role": "textfield", "text": "hi" } },
                { "tool": "desk_press_keys", "params": { "app": "Notes", "keys": "Enter" } },
                { "tool": "desk_click", "params": { "app": "Notes", "role": "button", "title": "Send" } },
            ]}),
        )
        .await;

    assert!(!flow.is_error, "{}", text_of(&flow));
}
