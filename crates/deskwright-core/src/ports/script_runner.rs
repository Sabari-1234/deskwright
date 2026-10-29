use crate::error::DeskwrightResult;
use crate::model::script::Script;

/// Runs automation scripts and returns their output.
pub trait ScriptRunner: Send + Sync {
    /// Runs the script, stopping it once its timeout passes.
    fn run(&self, script: &Script) -> DeskwrightResult<String>;
}
