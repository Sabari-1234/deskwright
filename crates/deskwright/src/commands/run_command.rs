use std::path::{Path, PathBuf};
use std::sync::Arc;

use deskwright::Deskwright;
use deskwright_runner::clock::SystemClock;
use deskwright_runner::flow::Flow;
use deskwright_runner::flow_runner::FlowRunner;
use deskwright_runner::report::FlowReport;
use deskwright_runner::reporters::{HtmlTraceReporter, JsonReporter, Reporter, TextReporter};

use crate::cli::ReportFormat;

/// What `run` asks for, besides the flows.
pub struct RunOptions<'options> {
    pub reporter: ReportFormat,
    pub trace: Option<&'options Path>,
    pub repeat: u32,
}

/// Runs every flow `repeat` times and returns the printed report and whether all passed.
pub async fn run(
    deskwright: &Deskwright,
    flow_paths: &[PathBuf],
    options: &RunOptions<'_>,
) -> anyhow::Result<(String, bool)> {
    let flows = load_flows(flow_paths)?;
    let runner = FlowRunner::new(
        deskwright.dispatcher(),
        Arc::new(SystemClock),
        options.trace.is_some(),
    );
    let mut reports = Vec::new();
    for _ in 0..options.repeat {
        for flow in &flows {
            reports.push(runner.run(flow).await);
        }
    }
    if let Some(trace_path) = options.trace {
        std::fs::write(trace_path, HtmlTraceReporter.render(&reports))?;
    }
    let has_passed = reports.iter().all(FlowReport::has_passed);
    Ok((reporter_for(options.reporter).render(&reports), has_passed))
}

fn reporter_for(format: ReportFormat) -> Box<dyn Reporter> {
    match format {
        ReportFormat::Text => Box::new(TextReporter),
        ReportFormat::Json => Box::new(JsonReporter),
    }
}

/// Reads and validates every flow before any runs, so a typo in the last file doesn't
/// surface after the first flows have already changed the desktop.
fn load_flows(paths: &[PathBuf]) -> anyhow::Result<Vec<Flow>> {
    paths
        .iter()
        .map(|path| {
            let contents = std::fs::read_to_string(path)
                .map_err(|error| anyhow::anyhow!("cannot read {}: {error}", path.display()))?;
            Flow::parse(&contents).map_err(|error| anyhow::anyhow!("{}: {error}", path.display()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_file_of_an_invalid_flow() {
        let directory = std::env::temp_dir().join(format!("deskwright-run-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let empty_flow = directory.join("empty.json");
        std::fs::write(&empty_flow, r#"{ "name": "x", "steps": [] }"#).unwrap();
        let missing = directory.join("missing.json");

        let invalid = load_flows(std::slice::from_ref(&empty_flow)).unwrap_err();
        let unreadable = load_flows(&[missing]).unwrap_err();

        assert!(
            invalid
                .to_string()
                .ends_with("empty.json: invalid flow: it has no steps")
        );
        assert!(unreadable.to_string().starts_with("cannot read "));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
