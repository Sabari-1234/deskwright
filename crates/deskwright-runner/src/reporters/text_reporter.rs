use super::{Reporter, format_milliseconds};
use crate::report::FlowReport;

/// Longest step output shown per line; the JSON and HTML reports keep the full text.
const MAX_OUTPUT_CHARACTERS: usize = 160;

/// A console summary: each flow and step with its timing, then the totals.
#[derive(Debug, Default, Clone, Copy)]
pub struct TextReporter;

impl Reporter for TextReporter {
    fn render(&self, reports: &[FlowReport]) -> String {
        let mut lines = Vec::new();
        for report in reports {
            let verdict = if report.has_passed() { "PASS" } else { "FAIL" };
            lines.push(format!(
                "{verdict} {} ({})",
                report.name,
                format_milliseconds(report.duration)
            ));
            for step in &report.steps {
                let status = if step.is_error { "failed" } else { "ok" };
                lines.push(format!(
                    "  {:>3}. {:<28} {:>10}  {status}: {}",
                    step.index + 1,
                    step.label,
                    format_milliseconds(step.duration),
                    one_line(&step.text)
                ));
            }
            if report.skipped_step_count > 0 {
                lines.push(format!(
                    "       {} step(s) skipped after the failure",
                    report.skipped_step_count
                ));
            }
        }
        let passed_count = reports.iter().filter(|report| report.has_passed()).count();
        lines.push(format!(
            "{passed_count} passed, {} failed",
            reports.len() - passed_count
        ));
        lines.join("\n") + "\n"
    }
}

fn one_line(text: &str) -> String {
    let flattened = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flattened.chars().count() <= MAX_OUTPUT_CHARACTERS {
        return flattened;
    }
    let shortened: String = flattened.chars().take(MAX_OUTPUT_CHARACTERS).collect();
    format!("{shortened}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reporters::test_reports::passing_and_failing;

    #[test]
    fn lists_flows_steps_timings_and_totals() {
        let text = TextReporter.render(&passing_and_failing());

        assert_eq!(
            text,
            "PASS Open Notes (98.0 ms)\n\
             \x20   1. launch                          97.4 ms  ok: Notes is frontmost.\n\
             FAIL Send <hi> (6.0 ms)\n\
             \x20   1. desk_click                       5.0 ms  failed: no element matching title 'send' appeared\n\
             \x20      2 step(s) skipped after the failure\n\
             1 passed, 1 failed\n"
        );
    }

    #[test]
    fn shortens_long_output_to_one_line() {
        let long = format!("first\nsecond {}", "x".repeat(200));

        let shortened = one_line(&long);

        assert!(shortened.starts_with("first second x"));
        assert!(shortened.ends_with('…'));
        assert_eq!(shortened.chars().count(), MAX_OUTPUT_CHARACTERS + 1);
    }
}
