use super::Reporter;
use crate::report::FlowReport;

/// Machine-readable results, for CI systems and other tools.
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonReporter;

impl Reporter for JsonReporter {
    fn render(&self, reports: &[FlowReport]) -> String {
        let document = serde_json::json!({
            "passed": reports.iter().filter(|report| report.has_passed()).count(),
            "failed": reports.iter().filter(|report| !report.has_passed()).count(),
            "flows": reports,
        });
        // Serializing plain data with string keys cannot fail.
        serde_json::to_string_pretty(&document).unwrap_or_default() + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reporters::test_reports::passing_and_failing;

    #[test]
    fn reports_totals_and_every_flow() {
        let json: serde_json::Value =
            serde_json::from_str(&JsonReporter.render(&passing_and_failing())).unwrap();

        assert_eq!(json["passed"], 1);
        assert_eq!(json["failed"], 1);
        assert_eq!(json["flows"][1]["skippedStepCount"], 2);
        assert_eq!(json["flows"][0]["steps"][0]["duration"], 97.4);
    }
}
