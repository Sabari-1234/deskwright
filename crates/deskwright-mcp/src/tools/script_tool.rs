use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use deskwright_core::model::script::{Script, ScriptLanguage};
use deskwright_core::ports::ScriptRunner;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLanguageArgument {
    #[default]
    AppleScript,
    JavaScript,
}

impl From<ScriptLanguageArgument> for ScriptLanguage {
    fn from(language: ScriptLanguageArgument) -> Self {
        match language {
            ScriptLanguageArgument::AppleScript => Self::AppleScript,
            ScriptLanguageArgument::JavaScript => Self::JavaScript,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct ScriptArguments {
    /// The script source.
    code: String,
    /// "applescript" (default) or "javascript" (JXA).
    #[serde(default)]
    language: ScriptLanguageArgument,
}

/// Runs AppleScript/JXA. Equivalent to running arbitrary commands as the user, so the
/// composition root registers it only when explicitly enabled.
pub struct ScriptTool {
    runner: Arc<dyn ScriptRunner>,
    timeout: Duration,
}

impl ScriptTool {
    pub fn new(runner: Arc<dyn ScriptRunner>, timeout: Duration) -> Self {
        Self { runner, timeout }
    }
}

#[async_trait]
impl TypedTool for ScriptTool {
    type Arguments = ScriptArguments;

    const NAME: &'static str = "desk_script";
    const DESCRIPTION: &'static str = "Run AppleScript or JavaScript for Automation (JXA) and \
        return its output. Scripts run with the user's full permissions and can do anything \
        the user can, so only run scripts the user asked for.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(&self, arguments: ScriptArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let script = match Script::new(arguments.language.into(), &arguments.code, self.timeout) {
            Ok(script) => script,
            Err(invalid_script) => return invalid_script.into(),
        };
        let runner = self.runner.clone();
        run_blocking(move || runner.run(&script))
            .await
            .map(|output| {
                if output.is_empty() {
                    ToolOutput::text("Script finished with no output.")
                } else {
                    ToolOutput::text(output)
                }
            })
            .into()
    }
}
