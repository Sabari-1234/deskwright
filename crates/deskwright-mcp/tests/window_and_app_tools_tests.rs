mod support;

use deskwright_core::model::geometry::Bounds;
use deskwright_core::ports::AppDirectory as _;
use deskwright_core::testing::element;
use serde_json::json;
use support::{Fixture, text_of};

/// The fixture's "Notes" app gets a second, positioned window titled "Groceries".
fn add_groceries_window(fixture: &Fixture) {
    let notes = fixture
        .desktop
        .running_apps()
        .unwrap()
        .into_iter()
        .find(|app| app.name == "Notes")
        .unwrap();
    let mut groceries = element("AXWindow", Some("Groceries"));
    groceries.bounds = Some(Bounds {
        x: 100.0,
        y: 50.0,
        width: 400.0,
        height: 300.0,
    });
    fixture.desktop.add_window(notes.process_id, groceries);
}

#[tokio::test]
async fn lists_windows_one_per_line() {
    let fixture = Fixture::new();
    add_groceries_window(&fixture);

    let output = fixture
        .call("desk_window", json!({ "action": "list" }))
        .await;

    assert_eq!(
        text_of(&output),
        "[0] \"Main\"\n[1] \"Groceries\" at 100,50 size 400x300"
    );
}

#[tokio::test]
async fn moves_resizes_and_minimizes_a_window_by_title() {
    let fixture = Fixture::new();
    add_groceries_window(&fixture);

    let moved = fixture
        .call(
            "desk_window",
            json!({ "action": "move", "title": "grocer", "x": 0, "y": 25 }),
        )
        .await;
    let resized = fixture
        .call(
            "desk_window",
            json!({ "action": "resize", "title": "grocer", "width": 800, "height": 600 }),
        )
        .await;
    let minimized = fixture
        .call("desk_window", json!({ "action": "minimize", "index": 1 }))
        .await;

    assert_eq!(
        text_of(&moved),
        "Moved [1] \"Groceries\" at 0,25 size 400x300."
    );
    assert_eq!(
        text_of(&resized),
        "Resized [1] \"Groceries\" at 0,25 size 800x600."
    );
    assert_eq!(
        text_of(&minimized),
        "Minimized [1] \"Groceries\" at 0,25 size 800x600 (minimized)."
    );
}

#[tokio::test]
async fn closes_raises_and_restores_windows() {
    let fixture = Fixture::new();
    add_groceries_window(&fixture);

    let raised = fixture
        .call("desk_window", json!({ "action": "raise", "app": "Notes" }))
        .await;
    let restored = fixture
        .call("desk_window", json!({ "action": "restore" }))
        .await;
    let closed = fixture
        .call(
            "desk_window",
            json!({ "action": "close", "title": "Groceries" }),
        )
        .await;
    let listed = fixture
        .call("desk_window", json!({ "action": "list" }))
        .await;

    assert_eq!(text_of(&raised), "Brought to the front [0] \"Main\".");
    assert_eq!(text_of(&restored), "Restored [0] \"Main\".");
    assert!(text_of(&closed).starts_with("Closed [1] \"Groceries\""));
    assert_eq!(text_of(&listed), "[0] \"Main\"");
}

#[tokio::test]
async fn rejects_incomplete_or_impossible_window_geometry() {
    let fixture = Fixture::new();

    let missing_y = fixture
        .call("desk_window", json!({ "action": "move", "x": 10 }))
        .await;
    let zero_width = fixture
        .call(
            "desk_window",
            json!({ "action": "resize", "width": 0, "height": 10 }),
        )
        .await;
    let unknown = fixture
        .call(
            "desk_window",
            json!({ "action": "close", "title": "Budget" }),
        )
        .await;

    assert!(text_of(&missing_y).contains("\"move\" needs y"));
    assert!(zero_width.is_error);
    assert!(text_of(&unknown).contains("Notes has no window titled \"Budget\""));
}

#[tokio::test]
async fn says_when_an_app_has_no_windows() {
    let fixture = Fixture::new();
    fixture.desktop.add_app("Calculator", None);

    let output = fixture
        .call(
            "desk_window",
            json!({ "action": "list", "app": "Calculator" }),
        )
        .await;

    assert_eq!(text_of(&output), "Calculator has no open windows.");
}

#[tokio::test]
async fn quits_an_app_and_reports_one_that_will_not_quit() {
    let fixture = Fixture::new();
    let stubborn = fixture.desktop.add_app("Pages", None);
    fixture.desktop.make_ignore_quit(stubborn);

    let quit = fixture
        .call("desk_quit_app", json!({ "app": "Notes" }))
        .await;
    let refused = fixture
        .call("desk_quit_app", json!({ "app": "Pages" }))
        .await;
    let forced = fixture
        .call("desk_quit_app", json!({ "app": "Pages", "force": true }))
        .await;

    assert_eq!(text_of(&quit), "Notes has quit.");
    assert!(refused.is_error);
    assert!(text_of(&refused).contains("asking to save changes"));
    assert_eq!(text_of(&forced), "Pages has quit.");
}

#[tokio::test]
async fn writes_and_reads_the_clipboard_without_echoing_on_write() {
    let fixture = Fixture::new();

    let empty = fixture.call("desk_read_clipboard", json!({})).await;
    let written = fixture
        .call("desk_write_clipboard", json!({ "text": "s3cret" }))
        .await;
    let read = fixture.call("desk_read_clipboard", json!({})).await;

    assert_eq!(text_of(&empty), "The clipboard holds no text.");
    assert_eq!(text_of(&written), "Put the text on the clipboard.");
    assert_eq!(text_of(&read), "s3cret");
}
