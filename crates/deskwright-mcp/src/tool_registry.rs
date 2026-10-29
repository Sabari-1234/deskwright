use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;

use crate::tool::{JsonObject, Tool, ToolContext, ToolDispatcher, ToolOutput, ToolSpec};

/// Two tools were registered under the same name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateToolName(pub &'static str);

impl fmt::Display for DuplicateToolName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "a tool named '{}' is already registered", self.0)
    }
}

impl Error for DuplicateToolName {}

/// Registry of tools by name. Adding a tool means registering one more type here; nothing
/// else dispatches on tool names.
#[derive(Default)]
pub struct ToolRegistry {
    tools_by_name: BTreeMap<&'static str, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: impl Tool + 'static) -> Result<&mut Self, DuplicateToolName> {
        let name = tool.spec().name;
        if self.tools_by_name.contains_key(name) {
            return Err(DuplicateToolName(name));
        }
        self.tools_by_name.insert(name, Arc::new(tool));
        Ok(self)
    }

    pub fn contains(&self, tool_name: &str) -> bool {
        self.tools_by_name.contains_key(tool_name)
    }

    pub fn spec(&self, tool_name: &str) -> Option<ToolSpec> {
        self.tools_by_name.get(tool_name).map(|tool| tool.spec())
    }

    /// Specs of all tools, sorted by name.
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools_by_name
            .values()
            .map(|tool| tool.spec())
            .collect()
    }
}

#[async_trait]
impl ToolDispatcher for ToolRegistry {
    async fn dispatch(&self, tool_name: &str, arguments: JsonObject) -> ToolOutput {
        let Some(tool) = self.tools_by_name.get(tool_name) else {
            return ToolOutput::error(format!("unknown tool '{tool_name}'"));
        };
        let started_at = Instant::now();
        let output = tool
            .call(arguments, &ToolContext { dispatcher: self })
            .await;
        // Arguments are never logged: they can contain typed passwords or script source.
        tracing::info!(
            tool = tool_name,
            is_error = output.is_error,
            elapsed_ms = started_at.elapsed().as_millis(),
            "tool call finished"
        );
        output
    }
}
