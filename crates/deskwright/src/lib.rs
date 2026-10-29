//! Deskwright as a Rust library, like Playwright's library API: [`Deskwright`] drives the
//! desktop through the same tools the MCP server offers.
//!
//! ```no_run
//! # async fn example() -> anyhow::Result<()> {
//! use deskwright::{Deskwright, DeskwrightConfiguration};
//! use serde_json::json;
//!
//! let deskwright = Deskwright::launch(&DeskwrightConfiguration::default())?;
//! let output = deskwright
//!     .call("desk_click", json!({ "app": "Notes", "role": "button", "title": "New Note" }))
//!     .await;
//! assert!(!output.is_error);
//! # Ok(())
//! # }
//! ```

mod composition;
mod configuration;
mod platform;

use std::sync::Arc;

use deskwright_mcp::tool::ToolDispatcher;
use deskwright_mcp::tool_registry::ToolRegistry;

pub use configuration::DeskwrightConfiguration;
pub use deskwright_mcp::tool::{OutputBlock, ToolOutput, ToolSpec};

/// A connection to this computer's desktop: every tool, wired to the native adapters.
#[derive(Clone)]
pub struct Deskwright {
    registry: Arc<ToolRegistry>,
}

impl Deskwright {
    /// Builds the native adapters and the tools. Fails on an unsupported operating system.
    pub fn launch(configuration: &DeskwrightConfiguration) -> anyhow::Result<Self> {
        let adapters = platform::native_adapters(configuration)?;
        Ok(Self::from_registry(composition::build_tool_registry(
            configuration,
            adapters,
        )?))
    }

    fn from_registry(registry: ToolRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    /// Runs one tool. `arguments` must be a JSON object, as in an MCP tool call.
    pub async fn call(&self, tool_name: &str, arguments: serde_json::Value) -> ToolOutput {
        let serde_json::Value::Object(arguments) = arguments else {
            return ToolOutput::error("tool arguments must be a JSON object");
        };
        self.registry.dispatch(tool_name, arguments).await
    }

    /// Every available tool, sorted by name.
    pub fn tools(&self) -> Vec<ToolSpec> {
        self.registry.specs()
    }

    /// The tools as a dispatcher, for the MCP server and the flow runner.
    pub fn dispatcher(&self) -> Arc<ToolRegistry> {
        self.registry.clone()
    }
}

impl std::fmt::Debug for Deskwright {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Deskwright")
            .field("tool_count", &self.registry.specs().len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use deskwright_core::testing::FakeDesktop;
    use serde_json::json;

    use super::*;

    fn fake_deskwright() -> Deskwright {
        let desktop = Arc::new(FakeDesktop::new());
        desktop.add_app("Notes", None);
        let registry = composition::build_tool_registry(
            &DeskwrightConfiguration::default(),
            platform::fake_adapters(desktop),
        )
        .unwrap();
        Deskwright::from_registry(registry)
    }

    #[tokio::test]
    async fn calls_tools_by_name_with_json_arguments() {
        let deskwright = fake_deskwright();

        let output = deskwright.call("desk_list_apps", json!({})).await;

        assert!(!output.is_error);
        assert!(matches!(&output.blocks[0], OutputBlock::Text(text) if text.contains("Notes")));
    }

    #[tokio::test]
    async fn refuses_arguments_that_are_not_an_object() {
        let output = fake_deskwright()
            .call("desk_list_apps", json!([1, 2]))
            .await;

        assert_eq!(
            output,
            ToolOutput::error("tool arguments must be a JSON object")
        );
    }

    #[test]
    fn lists_its_tools() {
        let deskwright = fake_deskwright();

        assert!(
            deskwright
                .tools()
                .iter()
                .any(|tool| tool.name == "desk_click")
        );
        assert!(format!("{deskwright:?}").starts_with("Deskwright { tool_count: "));
        assert!(deskwright.dispatcher().contains("desk_expect"));
    }
}
