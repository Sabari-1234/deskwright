mod support;

use deskwright_core::model::key_chord::KeyChord;
use deskwright_core::testing::RecordedAction;
use serde_json::{Value, json};
use support::{Fixture, text_of};

#[tokio::test]
async fn launch_brings_the_app_to_the_front_and_returns_it() {
    let fixture = Fixture::new();

    let launched = fixture
        .call(
            "desk_launch_app",
            json!({ "app": "Google Chrome", "url": "https://www.google.com/search?q=rust" }),
        )
        .await;

    assert!(!launched.is_error, "{}", text_of(&launched));
    let app: Value = serde_json::from_str(&text_of(&launched)).expect("JSON output");
    assert_eq!(app["name"], "Google Chrome");
    assert_eq!(app["isActive"], true);
    assert_eq!(
        fixture.desktop.recorded_actions(),
        [RecordedAction::OpenApp {
            name_or_bundle_id: "Google Chrome".to_owned(),
            address: Some("https://www.google.com/search?q=rust".to_owned()),
        }]
    );
}

#[tokio::test]
async fn launch_rejects_unsafe_addresses_and_missing_apps() {
    let fixture = Fixture::new();
    fixture.desktop.make_not_installed("Ghost");

    let unsafe_address = fixture
        .call(
            "desk_launch_app",
            json!({ "app": "Notes", "url": "file:///etc/passwd" }),
        )
        .await;
    let missing = fixture
        .call("desk_launch_app", json!({ "app": "Ghost" }))
        .await;

    assert!(text_of(&unsafe_address).contains("not an http or https address"));
    assert!(text_of(&missing).contains("no installed app matches 'Ghost'"));
    assert!(fixture.desktop.recorded_actions().is_empty());
}

#[tokio::test]
async fn press_keys_sends_the_chord_to_the_frontmost_target_app() {
    let fixture = Fixture::new();

    let pressed = fixture
        .call(
            "desk_press_keys",
            json!({ "keys": "cmd+shift+g", "app": "Notes" }),
        )
        .await;

    assert_eq!(text_of(&pressed), "Pressed Cmd+Shift+G in Notes.");
    assert_eq!(
        fixture.desktop.recorded_actions(),
        [RecordedAction::PressChord(
            KeyChord::parse("Cmd+Shift+G").unwrap()
        )]
    );
}

#[tokio::test]
async fn press_keys_refuses_a_background_app_and_unknown_keys() {
    let fixture = Fixture::new();
    fixture.desktop.add_app("Google Chrome", None);

    let background = fixture
        .call(
            "desk_press_keys",
            json!({ "keys": "Enter", "app": "Google Chrome" }),
        )
        .await;
    let unknown = fixture
        .call("desk_press_keys", json!({ "keys": "Hyper+S" }))
        .await;

    assert!(text_of(&background).contains("is not the frontmost app"));
    assert!(text_of(&unknown).contains("'hyper' is not a modifier"));
    assert!(fixture.desktop.recorded_actions().is_empty());
}

#[tokio::test]
async fn launch_then_press_keys_works_as_one_batch() {
    let fixture = Fixture::new();
    fixture.desktop.add_app("Google Chrome", None);

    let batch = fixture
        .call(
            "desk_batch",
            json!({ "actions": [
                { "tool": "desk_launch_app", "params": { "app": "Google Chrome" } },
                { "tool": "desk_press_keys", "params": { "keys": "Cmd+L", "app": "Google Chrome" } },
            ]}),
        )
        .await;

    assert!(!batch.is_error, "{}", text_of(&batch));
}

#[tokio::test]
async fn typing_without_a_ref_types_into_the_focused_element_of_the_target_app() {
    let fixture = Fixture::new();

    let typed = fixture
        .call("desk_type", json!({ "text": "hi", "app": "Notes" }))
        .await;

    assert_eq!(
        text_of(&typed),
        "Typed text into the focused element of Notes."
    );
    assert_eq!(
        fixture.desktop.recorded_actions(),
        [RecordedAction::TypeText("hi".to_owned())]
    );
}

#[tokio::test]
async fn typing_without_a_ref_refuses_a_background_app() {
    let fixture = Fixture::new();
    fixture.desktop.add_app("WhatsApp", None);

    let typed = fixture
        .call("desk_type", json!({ "text": "hi", "app": "WhatsApp" }))
        .await;

    assert!(text_of(&typed).contains("is not the frontmost app"));
    assert!(fixture.desktop.recorded_actions().is_empty());
}
