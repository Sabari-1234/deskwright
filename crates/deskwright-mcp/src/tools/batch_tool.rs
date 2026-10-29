use async_trait::async_trait;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::tool::{JsonObject, OutputBlock, ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

const MAX_BATCH_ACTIONS: usize = 50;

/// One queued tool call (Command pattern: the call is data the batch executes later).
#[derive(Deserialize, JsonSchema)]
pub struct BatchAction {
    /// Name of the tool to call, e.g. "desk_click".
    tool: String,
    /// That tool's arguments.
    #[serde(default)]
    params: JsonObject,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchArguments {
    /// Tool calls to run in order (at most 50).
    actions: Vec<BatchAction>,
    /// Keep going after a failed action (default false). Stopping is safer, because later
    /// actions usually depend on earlier ones.
    #[serde(default)]
    continue_on_error: bool,
}

/// Runs several tool calls in one round trip.
pub struct BatchTool;

#[async_trait]
impl TypedTool for BatchTool {
    type Arguments = BatchArguments;

    const NAME: &'static str = "desk_batch";
    const DESCRIPTION: &'static str = "Run a whole flow (up to 50 tool calls) on the \
        server in one request, like a Playwright script. Use locators (role/title/value) instead \
        of refs so each step finds and waits for its element, e.g. launch an app, click a \
        field, type, press Enter. Stops at the first failure unless continueOnError is true. \
        Batches cannot contain desk_batch.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(&self, arguments: BatchArguments, context: &ToolContext<'_>) -> ToolOutput {
        if let Some(rejection) = validate(&arguments.actions) {
            return rejection;
        }
        let mut blocks = Vec::new();
        let mut has_failure = false;
        for (index, action) in arguments.actions.into_iter().enumerate() {
            let output = context
                .dispatcher
                .dispatch(&action.tool, action.params)
                .await;
            let status = if output.is_error { "failed" } else { "ok" };
            blocks.push(OutputBlock::Text(format!(
                "[{index}] {} {status}:",
                action.tool
            )));
            blocks.extend(output.blocks);
            if output.is_error {
                has_failure = true;
                if !arguments.continue_on_error {
                    break;
                }
            }
        }
        ToolOutput {
            blocks,
            is_error: has_failure,
        }
    }
}

fn validate(actions: &[BatchAction]) -> Option<ToolOutput> {
    if actions.is_empty() {
        return Some(ToolOutput::error("a batch needs at least one action"));
    }
    if actions.len() > MAX_BATCH_ACTIONS {
        return Some(ToolOutput::error(format!(
            "a batch can hold at most {MAX_BATCH_ACTIONS} actions, got {}",
            actions.len()
        )));
    }
    actions
        .iter()
        .any(|action| action.tool == BatchTool::NAME)
        .then(|| ToolOutput::error("batches cannot contain desk_batch"))
}
