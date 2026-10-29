//! Runs flows: JSON files listing tool calls, like a Playwright test file. Each step is
//! dispatched through any [`deskwright_mcp::tool::ToolDispatcher`] (normally the tool registry),
//! timed, optionally followed by a screenshot, and collected into a [`report::FlowReport`]
//! that a [`reporters::Reporter`] renders as text, JSON or an HTML trace.

pub mod clock;
pub mod flow;
pub mod flow_runner;
pub mod report;
pub mod reporters;
