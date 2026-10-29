use deskwright_mcp::tool::JsonObject;
use serde::Deserialize;

/// Upper bound on steps per flow, so a malformed file can't run forever.
pub const MAX_FLOW_STEPS: usize = 500;

/// A named list of tool calls run in order, loaded from JSON:
///
/// ```json
/// { "name": "Send hi", "steps": [ { "tool": "desk_launch_app", "params": { "app": "Notes" } } ] }
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Flow {
    pub name: String,
    pub steps: Vec<FlowStep>,
    /// Run the remaining steps after a failure instead of stopping.
    #[serde(default)]
    pub continue_on_error: bool,
}

/// One tool call, in the same shape as a `desk_batch` action.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowStep {
    pub tool: String,
    #[serde(default)]
    pub params: JsonObject,
    /// Optional label for reports, like a Playwright `test.step` title.
    pub name: Option<String>,
}

impl FlowStep {
    /// The label shown in reports: the step's name, or its tool.
    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.tool)
    }
}

/// Why a flow file could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidFlow(pub String);

impl std::fmt::Display for InvalidFlow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid flow: {}", self.0)
    }
}

impl std::error::Error for InvalidFlow {}

impl Flow {
    /// Parses and validates a flow file's contents.
    pub fn parse(json: &str) -> Result<Self, InvalidFlow> {
        let flow: Self =
            serde_json::from_str(json).map_err(|error| InvalidFlow(error.to_string()))?;
        if flow.steps.is_empty() {
            return Err(InvalidFlow("it has no steps".to_owned()));
        }
        if flow.steps.len() > MAX_FLOW_STEPS {
            return Err(InvalidFlow(format!(
                "it has {} steps; at most {MAX_FLOW_STEPS} are allowed",
                flow.steps.len()
            )));
        }
        Ok(flow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_steps_with_optional_params_and_names() {
        let flow = Flow::parse(
            r#"{ "name": "Smoke", "steps": [
                { "tool": "desk_list_apps" },
                { "tool": "desk_click", "params": { "title": "Send" }, "name": "send it" }
            ] }"#,
        )
        .unwrap();

        assert_eq!(flow.name, "Smoke");
        assert!(!flow.continue_on_error);
        assert_eq!(flow.steps[0].label(), "desk_list_apps");
        assert_eq!(flow.steps[1].label(), "send it");
        assert_eq!(flow.steps[1].params["title"], "Send");
    }

    #[test]
    fn rejects_malformed_empty_and_oversized_flows() {
        let too_many = format!(
            r#"{{ "name": "Big", "steps": [{}] }}"#,
            vec![r#"{ "tool": "desk_list_apps" }"#; MAX_FLOW_STEPS + 1].join(",")
        );

        assert!(Flow::parse("not json").is_err());
        assert!(Flow::parse(r#"{ "name": "x", "steps": [], "extra": 1 }"#).is_err());
        assert_eq!(
            Flow::parse(r#"{ "name": "x", "steps": [] }"#),
            Err(InvalidFlow("it has no steps".to_owned()))
        );
        assert!(
            Flow::parse(&too_many)
                .unwrap_err()
                .to_string()
                .contains("at most 500")
        );
    }
}
