//! MCP delivery layer: each tool is its own type implementing [`typed_tool::TypedTool`],
//! tools are looked up by name in a [`tool_registry::ToolRegistry`], and
//! [`server::DeskwrightMcpServer`] adapts the registry to the `rmcp` protocol implementation.

mod blocking;
mod element_outline;
mod element_target_arguments;
mod match_limit;
mod output_format;
mod protocol_mapping;
mod query_arguments;
pub mod server;
mod timeout_argument;
pub mod tool;
mod tool_arguments;
pub mod tool_registry;
pub mod tools;
pub mod typed_tool;
