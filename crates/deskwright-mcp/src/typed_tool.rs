use async_trait::async_trait;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

use crate::tool::{JsonObject, Tool, ToolContext, ToolOutput, ToolSpec};
use crate::tool_arguments::{input_schema_for, parse_arguments};

/// A tool with typed arguments. Implement this instead of [`Tool`]: the blanket
/// implementation below derives the input schema from `Arguments` and parses incoming
/// arguments, so every tool validates its input the same way.
#[async_trait]
pub trait TypedTool: Send + Sync {
    type Arguments: DeserializeOwned + JsonSchema + Send;

    const NAME: &'static str;
    const DESCRIPTION: &'static str;
    const IS_READ_ONLY: bool;
    const IS_DESTRUCTIVE: bool;

    async fn run(&self, arguments: Self::Arguments, context: &ToolContext<'_>) -> ToolOutput;
}

#[async_trait]
impl<Typed: TypedTool> Tool for Typed {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: Typed::NAME,
            description: Typed::DESCRIPTION,
            input_schema: input_schema_for::<Typed::Arguments>(),
            is_read_only: Typed::IS_READ_ONLY,
            is_destructive: Typed::IS_DESTRUCTIVE,
        }
    }

    async fn call(&self, arguments: JsonObject, context: &ToolContext<'_>) -> ToolOutput {
        match parse_arguments::<Typed::Arguments>(arguments) {
            Ok(typed_arguments) => self.run(typed_arguments, context).await,
            Err(invalid_arguments) => invalid_arguments,
        }
    }
}
