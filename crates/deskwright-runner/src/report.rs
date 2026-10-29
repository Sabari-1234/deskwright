use std::time::Duration;

use serde::Serialize;

/// What one step did.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepReport {
    pub index: usize,
    pub label: String,
    pub tool: String,
    #[serde(serialize_with = "milliseconds")]
    pub duration: Duration,
    pub is_error: bool,
    /// The step's text output.
    pub text: String,
    /// PNG images from the step itself or from the trace screenshot after it, base64.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub images_base64: Vec<String>,
}

/// The result of running one flow.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowReport {
    pub name: String,
    pub steps: Vec<StepReport>,
    /// Steps not run because an earlier one failed.
    pub skipped_step_count: usize,
    #[serde(serialize_with = "milliseconds")]
    pub duration: Duration,
}

impl FlowReport {
    pub fn has_passed(&self) -> bool {
        self.skipped_step_count == 0 && self.steps.iter().all(|step| !step.is_error)
    }

    pub fn failed_step_count(&self) -> usize {
        self.steps.iter().filter(|step| step.is_error).count()
    }
}

fn milliseconds<S: serde::Serializer>(
    duration: &Duration,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_f64(duration.as_secs_f64() * 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn step(index: usize, is_error: bool) -> StepReport {
        StepReport {
            index,
            label: format!("step {index}"),
            tool: "desk_click".to_owned(),
            duration: Duration::from_millis(12),
            is_error,
            text: "Clicked e2.".to_owned(),
            images_base64: Vec::new(),
        }
    }

    fn report(steps: Vec<StepReport>, skipped_step_count: usize) -> FlowReport {
        FlowReport {
            name: "Flow".to_owned(),
            steps,
            skipped_step_count,
            duration: Duration::from_millis(30),
        }
    }

    #[test]
    fn passes_only_when_every_step_ran_and_succeeded() {
        assert!(report(vec![step(0, false)], 0).has_passed());
        assert!(!report(vec![step(0, true)], 0).has_passed());
        assert!(!report(vec![step(0, false)], 1).has_passed());
        assert_eq!(
            report(vec![step(0, true), step(1, true)], 0).failed_step_count(),
            2
        );
    }

    #[test]
    fn serializes_durations_in_milliseconds() {
        let json = serde_json::to_value(report(vec![step(0, false)], 0)).unwrap();

        assert_eq!(json["duration"], 30.0);
        assert_eq!(json["steps"][0]["duration"], 12.0);
        assert!(json["steps"][0].get("imagesBase64").is_none());
    }
}
