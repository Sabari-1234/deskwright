mod support;

use deskwright_core::testing::RecordedAction;
use serde_json::{Value, json};
use support::{Fixture, text_of};

#[tokio::test]
async fn list_apps_returns_running_apps_as_json() {
    let fixture = Fixture::new();

    let listed = fixture.call("desk_list_apps", json!({})).await;

    let apps: Value = serde_json::from_str(&text_of(&listed)).expect("JSON output");
    assert_eq!(apps[0]["name"], "Notes");
    assert_eq!(apps[0]["bundleId"], "com.apple.Notes");
    assert_eq!(apps[0]["isActive"], true);
}

#[tokio::test]
async fn menu_presses_the_item_at_the_given_path() {
    let fixture = Fixture::new();

    let chosen = fixture
        .call("desk_menu", json!({ "path": "File > Export…" }))
        .await;

    assert_eq!(text_of(&chosen), "Chose menu item 'File > Export…'.");
    assert_eq!(
        fixture.desktop.recorded_actions(),
        [RecordedAction::Press(fixture.export_menu_item)]
    );
}

#[tokio::test]
async fn menu_rejects_malformed_paths_and_missing_items() {
    let fixture = Fixture::new();

    let malformed = fixture.call("desk_menu", json!({ "path": "File >" })).await;
    let missing = fixture
        .call("desk_menu", json!({ "path": "File > Print" }))
        .await;

    assert!(text_of(&malformed).contains("is not a menu path"));
    assert!(text_of(&missing).contains("'Print' was not found"));
}
