use super::{Reporter, format_milliseconds};
use crate::report::{FlowReport, StepReport};

const STYLE: &str = "\
:root{--bg:#fff;--fg:#1d1d1f;--muted:#6e6e73;--line:#d2d2d7;--pass:#1a7f37;--fail:#cf222e;--card:#f5f5f7}\
@media (prefers-color-scheme:dark){:root{--bg:#1c1c1e;--fg:#f5f5f7;--muted:#a1a1a6;--line:#3a3a3c;--pass:#3fb950;--fail:#f85149;--card:#2c2c2e}}\
body{margin:0;padding:24px 16px;background:var(--bg);color:var(--fg);font:14px/1.5 -apple-system,system-ui,sans-serif}\
main{max-width:960px;margin:0 auto}h1{font-size:20px}h2{font-size:16px;margin:28px 0 8px}\
.pass{color:var(--pass)}.fail{color:var(--fail)}.muted{color:var(--muted)}\
details{border:1px solid var(--line);border-radius:8px;margin:6px 0;background:var(--card)}\
summary{cursor:pointer;padding:8px 12px;display:flex;gap:12px}summary .time{margin-left:auto;color:var(--muted)}\
pre{margin:0;padding:8px 12px;white-space:pre-wrap;word-break:break-word;border-top:1px solid var(--line)}\
img{display:block;max-width:100%;border-top:1px solid var(--line)}";

/// A self-contained HTML trace, a small version of Playwright's trace viewer: every step
/// with its timing, full output and the screenshot taken after it.
#[derive(Debug, Default, Clone, Copy)]
pub struct HtmlTraceReporter;

impl Reporter for HtmlTraceReporter {
    fn render(&self, reports: &[FlowReport]) -> String {
        let passed_count = reports.iter().filter(|report| report.has_passed()).count();
        let flows: String = reports.iter().map(render_flow).collect();
        format!(
            "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
             <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
             <title>Deskwright trace</title><style>{STYLE}</style></head><body><main>\
             <h1>Deskwright trace</h1><p>{passed_count} passed, {} failed</p>{flows}\
             </main></body></html>\n",
            reports.len() - passed_count
        )
    }
}

fn render_flow(report: &FlowReport) -> String {
    let (class, verdict) = if report.has_passed() {
        ("pass", "PASS")
    } else {
        ("fail", "FAIL")
    };
    let steps: String = report.steps.iter().map(render_step).collect();
    let skipped = if report.skipped_step_count > 0 {
        format!(
            "<p class=\"muted\">{} step(s) skipped after the failure</p>",
            report.skipped_step_count
        )
    } else {
        String::new()
    };
    format!(
        "<h2><span class=\"{class}\">{verdict}</span> {} <span class=\"muted\">{}</span></h2>{steps}{skipped}",
        escape_html(&report.name),
        format_milliseconds(report.duration)
    )
}

fn render_step(step: &StepReport) -> String {
    let (class, status) = if step.is_error {
        ("fail", "failed")
    } else {
        ("pass", "ok")
    };
    let images: String = step
        .images_base64
        .iter()
        .map(|image| {
            format!(
                "<img alt=\"Screen after step {}\" src=\"data:image/png;base64,{image}\">",
                step.index + 1
            )
        })
        .collect();
    let open = if step.is_error { " open" } else { "" };
    format!(
        "<details{open}><summary><span>{}.</span><span>{}</span><span class=\"{class}\">{status}</span>\
         <span class=\"time\">{}</span></summary><pre>{}</pre>{images}</details>",
        step.index + 1,
        escape_html(&step.label),
        format_milliseconds(step.duration),
        escape_html(&step.text)
    )
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reporters::test_reports::passing_and_failing;

    #[test]
    fn renders_every_step_with_its_screenshot_and_escapes_text() {
        let html = HtmlTraceReporter.render(&passing_and_failing());

        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<p>1 passed, 1 failed</p>"));
        assert!(html.contains("Send &lt;hi&gt;"));
        assert!(html.contains("src=\"data:image/png;base64,iVBORw0KGgo=\""));
        assert!(html.contains("<details open>"));
        assert!(html.contains("2 step(s) skipped after the failure"));
        assert!(html.contains("97.4 ms"));
    }

    #[test]
    fn escapes_every_special_character() {
        assert_eq!(
            escape_html("a & \"b\" <c>"),
            "a &amp; &quot;b&quot; &lt;c&gt;"
        );
    }
}
