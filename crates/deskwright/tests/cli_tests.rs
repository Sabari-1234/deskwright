//! Runs the real binary's command-line interface. Uses only tools that need no macOS
//! permissions (listing apps and tools), so it passes on any supported machine.

use std::path::PathBuf;
use std::process::{Command, Output};

fn deskwright(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deskwright"))
        .args(arguments)
        .env("RUST_LOG", "warn")
        .output()
        .expect("the binary runs")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A scratch folder for this test process, removed by the caller.
fn scratch_directory(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("deskwright-cli-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn lists_tools() {
    let output = deskwright(&["tools"]);

    assert!(output.status.success());
    assert!(stdout_of(&output).contains("desk_click "));
    assert!(!stdout_of(&output).contains("desk_script"));
}

#[test]
fn calls_one_tool_and_reports_failures_with_the_exit_code() {
    let listed = deskwright(&["call", "desk_list_apps"]);
    let unknown = deskwright(&["call", "desk_teleport"]);
    let bad_json = deskwright(&["call", "desk_list_apps", "{not json"]);

    assert!(listed.status.success());
    assert!(stdout_of(&listed).contains("\"processId\""));
    assert!(!unknown.status.success());
    assert!(stdout_of(&unknown).contains("unknown tool 'desk_teleport'"));
    assert!(!bad_json.status.success());
    assert!(String::from_utf8_lossy(&bad_json.stderr).contains("not valid JSON"));
}

#[test]
fn runs_flows_and_writes_reports() {
    let directory = scratch_directory("run");
    let passing = directory.join("passing.json");
    std::fs::write(
        &passing,
        r#"{ "name": "List apps", "steps": [ { "tool": "desk_list_apps", "name": "list" } ] }"#,
    )
    .unwrap();
    let failing = directory.join("failing.json");
    std::fs::write(
        &failing,
        r#"{ "name": "Broken", "steps": [ { "tool": "desk_teleport" }, { "tool": "desk_list_apps" } ] }"#,
    )
    .unwrap();
    let passing_path = passing.to_str().unwrap();

    let text = deskwright(&["run", passing_path, "--repeat", "2"]);
    let json = deskwright(&["run", passing_path, "--reporter", "json"]);
    let failed = deskwright(&["run", passing_path, failing.to_str().unwrap()]);

    assert!(text.status.success());
    assert!(stdout_of(&text).starts_with("PASS List apps ("));
    assert!(stdout_of(&text).ends_with("2 passed, 0 failed\n"));
    let report: serde_json::Value = serde_json::from_str(&stdout_of(&json)).unwrap();
    assert_eq!(report["passed"], 1);
    assert!(!failed.status.success());
    assert!(stdout_of(&failed).contains("1 step(s) skipped after the failure"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn writes_an_html_trace() {
    let directory = scratch_directory("trace");
    let flow = directory.join("flow.json");
    std::fs::write(
        &flow,
        r#"{ "name": "Trace me", "steps": [ { "tool": "desk_list_apps" } ] }"#,
    )
    .unwrap();
    let trace = directory.join("trace.html");

    let output = deskwright(&[
        "run",
        flow.to_str().unwrap(),
        "--trace",
        trace.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let html = std::fs::read_to_string(&trace).unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("Trace me"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn refuses_an_invalid_flow_before_running_anything() {
    let directory = scratch_directory("invalid");
    let flow = directory.join("empty.json");
    std::fs::write(&flow, r#"{ "name": "Empty", "steps": [] }"#).unwrap();

    let output = deskwright(&["run", flow.to_str().unwrap()]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("it has no steps"));
    std::fs::remove_dir_all(directory).unwrap();
}
