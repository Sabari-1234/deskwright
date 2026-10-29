mod support;

use deskwright_core::model::geometry::{Bounds, Point};
use deskwright_core::testing::{FakeElementId, RecordedAction, element};
use serde_json::json;
use support::{Fixture, MESSAGE_FIELD_REF, SEND_BUTTON_REF, text_of};

/// Adds a window to the fixture's frontmost app holding `role` titled `title`.
fn add_control(fixture: &Fixture, role: &str, title: &str) -> FakeElementId {
    let process_id = fixture.desktop.add_app("Settings", None);
    fixture.desktop.make_frontmost(process_id);
    let window = fixture
        .desktop
        .add_window(process_id, element("AXWindow", Some("Settings")));
    let mut attributes = element(role, Some(title));
    attributes.bounds = Some(Bounds {
        x: 10.0,
        y: 10.0,
        width: 20.0,
        height: 20.0,
    });
    fixture.desktop.add_child(window, attributes)
}

#[tokio::test]
async fn click_double_and_right_clicks_on_request() {
    let fixture = Fixture::new();
    fixture.call("desk_snapshot", json!({})).await;

    let double = fixture
        .call(
            "desk_click",
            json!({ "ref": SEND_BUTTON_REF, "clickCount": 2 }),
        )
        .await;
    let right = fixture
        .call(
            "desk_click",
            json!({ "ref": SEND_BUTTON_REF, "button": "right" }),
        )
        .await;

    assert_eq!(text_of(&double), "Clicked e2 using pointer double-click.");
    assert_eq!(text_of(&right), "Clicked e2 using accessibility menu.");
    assert_eq!(
        fixture.desktop.recorded_actions()[0],
        RecordedAction::DoubleClickAt(Point { x: 20.0, y: 10.0 })
    );
}

#[tokio::test]
async fn click_rejects_impossible_click_combinations() {
    let fixture = Fixture::new();

    let right_double = fixture
        .call(
            "desk_click",
            json!({ "ref": SEND_BUTTON_REF, "button": "right", "clickCount": 2 }),
        )
        .await;
    let triple = fixture
        .call(
            "desk_click",
            json!({ "ref": SEND_BUTTON_REF, "clickCount": 3 }),
        )
        .await;

    assert!(right_double.is_error);
    assert!(text_of(&right_double).contains("single click"));
    assert!(triple.is_error);
    assert!(text_of(&triple).contains("1 or 2"));
}

#[tokio::test]
async fn hover_moves_the_pointer_to_the_element() {
    let fixture = Fixture::new();

    let output = fixture
        .call("desk_hover", json!({ "role": "button", "title": "Send" }))
        .await;

    assert_eq!(text_of(&output), "Hovering over e2.");
    assert_eq!(
        fixture.desktop.recorded_actions(),
        [RecordedAction::MovePointerTo(Point { x: 20.0, y: 10.0 })]
    );
}

#[tokio::test]
async fn hover_and_scroll_require_an_element() {
    let fixture = Fixture::new();

    assert!(fixture.call("desk_hover", json!({})).await.is_error);
    assert!(
        fixture
            .call("desk_scroll_into_view", json!({}))
            .await
            .is_error
    );
    assert!(fixture.call("desk_check", json!({})).await.is_error);
    assert!(
        fixture
            .call("desk_select", json!({ "option": "PNG" }))
            .await
            .is_error
    );
}

#[tokio::test]
async fn drag_moves_one_element_onto_another() {
    let fixture = Fixture::new();
    add_control(&fixture, "AXGroup", "Drop zone");

    let output = fixture
        .call(
            "desk_drag",
            json!({
                "from": { "role": "button", "title": "Send", "app": "Notes" },
                "to": { "title": "Drop zone", "app": "Settings" },
            }),
        )
        .await;

    assert_eq!(
        text_of(&output),
        "Dragged the element onto the drop target."
    );
    assert_eq!(
        fixture.desktop.recorded_actions(),
        [RecordedAction::Drag {
            from: Point { x: 20.0, y: 10.0 },
            to: Point { x: 20.0, y: 20.0 },
        }]
    );
}

#[tokio::test]
async fn drag_requires_both_ends() {
    let fixture = Fixture::new();

    let output = fixture
        .call(
            "desk_drag",
            json!({ "from": { "ref": SEND_BUTTON_REF }, "to": {} }),
        )
        .await;

    assert!(output.is_error);
}

#[tokio::test]
async fn check_toggles_only_when_needed() {
    let fixture = Fixture::new();
    let wifi = add_control(&fixture, "AXCheckBox", "Wi-Fi");
    fixture.desktop.set_value(wifi, "0");

    let checked = fixture
        .call(
            "desk_check",
            json!({ "role": "checkbox", "title": "Wi-Fi" }),
        )
        .await;
    let again = fixture
        .call(
            "desk_check",
            json!({ "role": "checkbox", "title": "Wi-Fi" }),
        )
        .await;
    let unchecked = fixture
        .call(
            "desk_check",
            json!({ "role": "checkbox", "title": "Wi-Fi", "checked": false }),
        )
        .await;

    assert!(text_of(&checked).ends_with("is now checked (using accessibility press)."));
    assert!(text_of(&again).ends_with("was already checked."));
    assert!(text_of(&unchecked).ends_with("is now unchecked (using accessibility press)."));
}

#[tokio::test]
async fn select_picks_an_option_from_a_pop_up() {
    let fixture = Fixture::new();
    let format = add_control(&fixture, "AXPopUpButton", "Format");
    let menu = fixture.desktop.add_child(format, element("AXMenu", None));
    let png = fixture
        .desktop
        .add_child(menu, element("AXMenuItem", Some("PNG")));

    let output = fixture
        .call(
            "desk_select",
            json!({ "role": "popupbutton", "title": "Format", "option": "PNG" }),
        )
        .await;
    let missing = fixture
        .call(
            "desk_select",
            json!({ "role": "popupbutton", "title": "Format", "option": "GIF" }),
        )
        .await;

    assert!(text_of(&output).starts_with("Selected \"PNG\" in e"));
    assert!(
        fixture
            .desktop
            .recorded_actions()
            .contains(&RecordedAction::Press(png))
    );
    assert!(missing.is_error);
    assert!(text_of(&missing).contains("available: PNG"));
}

#[tokio::test]
async fn scroll_into_view_scrolls_the_element() {
    let fixture = Fixture::new();
    fixture.call("desk_snapshot", json!({})).await;

    let output = fixture
        .call("desk_scroll_into_view", json!({ "ref": MESSAGE_FIELD_REF }))
        .await;

    assert_eq!(text_of(&output), "Scrolled e3 into view.");
    assert!(matches!(
        fixture.desktop.recorded_actions()[..],
        [RecordedAction::ScrollIntoView(_)]
    ));
}

#[tokio::test]
async fn locators_can_be_scoped_to_a_container() {
    let fixture = Fixture::new();
    let settings_button = add_control(&fixture, "AXButton", "Send");

    let unscoped = fixture
        .call(
            "desk_find",
            json!({ "role": "button", "title": "Send", "app": "Notes" }),
        )
        .await;
    let scoped = fixture
        .call(
            "desk_click",
            json!({
                "role": "button",
                "title": "Send",
                "app": "Settings",
                "within": { "role": "window", "title": "Settings" },
            }),
        )
        .await;
    let outside = fixture
        .call(
            "desk_find",
            json!({
                "role": "button",
                "app": "Settings",
                "within": { "role": "toolbar" },
            }),
        )
        .await;

    assert!(text_of(&unscoped).contains("Button \"Send\""));
    assert!(text_of(&scoped).starts_with("Clicked e"));
    assert!(
        fixture
            .desktop
            .recorded_actions()
            .contains(&RecordedAction::Press(settings_button))
    );
    assert_eq!(text_of(&outside), "No matching elements.");
}
