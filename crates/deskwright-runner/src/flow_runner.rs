use std::sync::Arc;

use deskwright_mcp::tool::{JsonObject, OutputBlock, ToolDispatcher, ToolOutput};

use crate::clock::Clock;
use crate::flow::{Flow, FlowStep};
use crate::report::{FlowReport, StepImage, StepReport};

/// The tool called after each step when tracing, to show the screen as it was.
const TRACE_SCREENSHOT_TOOL: &str = "desk_screenshot";
/// Trace screenshots are JPEG at point resolution: readable, and a fraction of a full PNG's
/// size, so traces of long flows stay small.
const TRACE_SCREENSHOT_FORMAT: &str = "jpeg";

/// Runs a flow's steps in order through a tool dispatcher and reports each one.
pub struct FlowRunner {
    dispatcher: Arc<dyn ToolDispatcher>,
    clock: Arc<dyn Clock>,
    is_tracing: bool,
}

impl FlowRunner {
    /// With `is_tracing`, a screenshot is taken after every step for the trace report.
    pub fn new(
        dispatcher: Arc<dyn ToolDispatcher>,
        clock: Arc<dyn Clock>,
        is_tracing: bool,
    ) -> Self {
        Self {
            dispatcher,
            clock,
            is_tracing,
        }
    }

    pub async fn run(&self, flow: &Flow) -> FlowReport {
        let started = self.clock.now();
        let mut steps = Vec::with_capacity(flow.steps.len());
        for (index, step) in flow.steps.iter().enumerate() {
            let report = self.run_step(index, step).await;
            let should_stop = report.is_error && !flow.continue_on_error;
            steps.push(report);
            if should_stop {
                break;
            }
        }
        FlowReport {
            name: flow.name.clone(),
            skipped_step_count: flow.steps.len() - steps.len(),
            steps,
            duration: self.clock.now().saturating_duration_since(started),
        }
    }

    async fn run_step(&self, index: usize, step: &FlowStep) -> StepReport {
        let started = self.clock.now();
        let output = self
            .dispatcher
            .dispatch(&step.tool, step.params.clone())
            .await;
        let duration = self.clock.now().saturating_duration_since(started);
        let mut images = images_of(&output);
        if self.is_tracing {
            let arguments =
                JsonObject::from_iter([("format".to_owned(), TRACE_SCREENSHOT_FORMAT.into())]);
            let screenshot = self
                .dispatcher
                .dispatch(TRACE_SCREENSHOT_TOOL, arguments)
                .await;
            images.extend(images_of(&screenshot));
        }
        StepReport {
            index,
            label: step.label().to_owned(),
            tool: step.tool.clone(),
            duration,
            is_error: output.is_error,
            text: text_of(&output),
            images,
        }
    }
}

fn text_of(output: &ToolOutput) -> String {
    output
        .blocks
        .iter()
        .filter_map(|block| match block {
            OutputBlock::Text(text) => Some(text.as_str()),
            OutputBlock::Image { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn images_of(output: &ToolOutput) -> Vec<StepImage> {
    output
        .blocks
        .iter()
        .filter_map(|block| match block {
            OutputBlock::Image {
                base64_data,
                mime_type,
            } => Some(StepImage {
                mime_type: (*mime_type).to_owned(),
                base64_data: base64_data.clone(),
            }),
            OutputBlock::Text(_) => None,
        })
        .collect()
}
