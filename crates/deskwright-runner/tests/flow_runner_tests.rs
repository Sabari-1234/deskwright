use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use deskwright_mcp::tool::{JsonObject, OutputBlock, ToolDispatcher, ToolOutput};
use deskwright_runner::clock::Clock;
use deskwright_runner::flow::Flow;
use deskwright_runner::flow_runner::FlowRunner;

/// A clock that moves 10 ms forward every time it is read.
struct TickingClock {
    start: Instant,
    reads: Mutex<u32>,
}

impl TickingClock {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            reads: Mutex::new(0),
        }
    }
}

impl Clock for TickingClock {
    fn now(&self) -> Instant {
        let mut reads = self.reads.lock().unwrap();
        *reads += 1;
        self.start + Duration::from_millis(10) * *reads
    }
}

/// Answers every tool with a text, fails `desk_fail`, and returns an image for screenshots.
#[derive(Default)]
struct ScriptedDispatcher {
    calls: Mutex<Vec<String>>,
    trace_arguments: Mutex<Vec<serde_json::Value>>,
}

#[async_trait]
impl ToolDispatcher for ScriptedDispatcher {
    async fn dispatch(&self, tool_name: &str, arguments: JsonObject) -> ToolOutput {
        self.calls.lock().unwrap().push(tool_name.to_owned());
        if tool_name == "desk_screenshot" {
            self.trace_arguments
                .lock()
                .unwrap()
                .push(serde_json::Value::Object(arguments));
        }
        match tool_name {
            "desk_fail" => ToolOutput::error("it broke"),
            "desk_screenshot" => ToolOutput {
                blocks: vec![OutputBlock::Image {
                    base64_data: "UE5H".to_owned(),
                    mime_type: "image/jpeg",
                }],
                is_error: false,
            },
            other => ToolOutput::text(format!("{other} done")),
        }
    }
}

fn flow(tools: &[&str], continue_on_error: bool) -> Flow {
    let steps: Vec<String> = tools
        .iter()
        .map(|tool| format!(r#"{{ "tool": "{tool}" }}"#))
        .collect();
    Flow::parse(&format!(
        r#"{{ "name": "Flow", "continueOnError": {continue_on_error}, "steps": [{}] }}"#,
        steps.join(",")
    ))
    .unwrap()
}

fn runner(dispatcher: &Arc<ScriptedDispatcher>, is_tracing: bool) -> FlowRunner {
    FlowRunner::new(
        dispatcher.clone(),
        Arc::new(TickingClock::new()),
        is_tracing,
    )
}

#[tokio::test]
async fn runs_every_step_in_order_and_times_each_one() {
    let dispatcher = Arc::new(ScriptedDispatcher::default());

    let report = runner(&dispatcher, false)
        .run(&flow(&["desk_list_apps", "desk_snapshot"], false))
        .await;

    assert!(report.has_passed());
    assert_eq!(report.steps[1].text, "desk_snapshot done");
    assert_eq!(report.steps[0].duration, Duration::from_millis(10));
    assert_eq!(report.duration, Duration::from_millis(50));
    assert_eq!(
        *dispatcher.calls.lock().unwrap(),
        ["desk_list_apps", "desk_snapshot"]
    );
}

#[tokio::test]
async fn stops_at_the_first_failure_unless_told_to_continue() {
    let dispatcher = Arc::new(ScriptedDispatcher::default());
    let steps = ["desk_list_apps", "desk_fail", "desk_snapshot"];

    let stopped = runner(&dispatcher, false).run(&flow(&steps, false)).await;
    let continued = runner(&dispatcher, false).run(&flow(&steps, true)).await;

    assert!(!stopped.has_passed());
    assert_eq!(stopped.steps.len(), 2);
    assert_eq!(stopped.skipped_step_count, 1);
    assert_eq!(stopped.steps[1].text, "it broke");
    assert_eq!(continued.steps.len(), 3);
    assert_eq!(continued.failed_step_count(), 1);
}

#[tokio::test]
async fn tracing_adds_a_screenshot_after_each_step() {
    let dispatcher = Arc::new(ScriptedDispatcher::default());

    let report = runner(&dispatcher, true)
        .run(&flow(&["desk_list_apps"], false))
        .await;

    assert_eq!(report.steps[0].images[0].base64_data, "UE5H");
    assert_eq!(report.steps[0].images[0].mime_type, "image/jpeg");
    assert_eq!(
        *dispatcher.trace_arguments.lock().unwrap(),
        [serde_json::json!({ "format": "jpeg" })]
    );
    assert_eq!(
        *dispatcher.calls.lock().unwrap(),
        ["desk_list_apps", "desk_screenshot"]
    );
}
