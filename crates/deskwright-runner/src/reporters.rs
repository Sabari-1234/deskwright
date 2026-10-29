//! Renderers for flow results (**Strategy**): the CLI picks one, and a new format is a new
//! type.

mod html_trace_reporter;
mod json_reporter;
mod text_reporter;

pub use html_trace_reporter::HtmlTraceReporter;
pub use json_reporter::JsonReporter;
pub use text_reporter::TextReporter;

use crate::report::FlowReport;

/// Turns the reports of one run into a document.
pub trait Reporter {
    fn render(&self, reports: &[FlowReport]) -> String;
}

/// Milliseconds with one decimal, for human-readable timings.
fn format_milliseconds(duration: std::time::Duration) -> String {
    format!("{:.1} ms", duration.as_secs_f64() * 1000.0)
}

#[cfg(test)]
pub(crate) mod test_reports {
    use std::time::Duration;

    use crate::report::{FlowReport, StepImage, StepReport};

    pub fn passing_and_failing() -> Vec<FlowReport> {
        vec![
            FlowReport {
                name: "Open Notes".to_owned(),
                steps: vec![StepReport {
                    index: 0,
                    label: "launch".to_owned(),
                    tool: "desk_launch_app".to_owned(),
                    duration: Duration::from_micros(97_400),
                    is_error: false,
                    text: "Notes is frontmost.".to_owned(),
                    images: vec![StepImage {
                        mime_type: "image/jpeg".to_owned(),
                        base64_data: "/9j/4AAQ".to_owned(),
                    }],
                }],
                skipped_step_count: 0,
                duration: Duration::from_millis(98),
            },
            FlowReport {
                name: "Send <hi>".to_owned(),
                steps: vec![StepReport {
                    index: 0,
                    label: "desk_click".to_owned(),
                    tool: "desk_click".to_owned(),
                    duration: Duration::from_millis(5),
                    is_error: true,
                    text: "no element matching title 'send' appeared".to_owned(),
                    images: Vec::new(),
                }],
                skipped_step_count: 2,
                duration: Duration::from_millis(6),
            },
        ]
    }
}
