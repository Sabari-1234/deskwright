mod support;

use deskwright_core::DeskwrightError;
use deskwright_core::testing::RecordedAction;
use serde_json::json;
use support::{Fixture, text_of};

#[tokio::test]
async fn returns_the_script_output() {
    let fixture = Fixture::new();
    fixture.desktop.set_script_result(Ok("42".to_owned()));

    let output = fixture
        .call(
            "desk_script",
            json!({ "code": "6 * 7", "language": "javascript" }),
        )
        .await;

    assert_eq!(text_of(&output), "42");
    assert_eq!(
        fixture.desktop.recorded_actions(),
        [RecordedAction::RunScript("6 * 7".to_owned())]
    );
}

#[tokio::test]
async fn says_so_when_a_script_prints_nothing() {
    let fixture = Fixture::new();

    let output = fixture.call("desk_script", json!({ "code": "beep" })).await;

    assert_eq!(text_of(&output), "Script finished with no output.");
}

#[tokio::test]
async fn reports_script_failures_as_errors() {
    let fixture = Fixture::new();
    fixture
        .desktop
        .set_script_result(Err(DeskwrightError::ScriptFailed(
            "syntax error".to_owned(),
        )));

    let output = fixture.call("desk_script", json!({ "code": "oops" })).await;

    assert!(output.is_error);
    assert!(text_of(&output).contains("syntax error"));
}

#[tokio::test]
async fn rejects_blank_scripts_and_unknown_languages() {
    let fixture = Fixture::new();

    let blank = fixture.call("desk_script", json!({ "code": "  " })).await;
    let unknown_language = fixture
        .call("desk_script", json!({ "code": "x", "language": "python" }))
        .await;

    assert!(text_of(&blank).contains("script source is empty"));
    assert!(text_of(&unknown_language).contains("invalid arguments"));
    assert!(fixture.desktop.recorded_actions().is_empty());
}
