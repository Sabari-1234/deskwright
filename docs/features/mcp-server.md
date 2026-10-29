# MCP Server and Tool Registry

> How tools are defined, registered, and served over the Model Context Protocol.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

This is the delivery layer every feature goes through. It keeps protocol details out of
the tools and tool details out of the protocol adapter, so tools can be added without
touching the server.

## How it works

1. Each tool is one type implementing `TypedTool`. It declares its typed `Arguments`
   (serde + schemars), name, description and read-only/destructive flags, and a `run` method
   that calls one core service.
2. A blanket implementation turns every `TypedTool` into a `Tool`. It derives the JSON input
   schema from `Arguments` (field doc comments become descriptions) and parses incoming
   arguments, returning a readable error result when they're invalid.
3. `ToolRegistry` stores tools by name, rejects duplicate names, and implements
   `ToolDispatcher`. Each call is logged with name, success and duration, never arguments.
4. `DeskwrightMcpServer` implements `rmcp`'s `ServerHandler`: `list_tools` maps tool specs to MCP
   tools with annotations, `call_tool` dispatches to the registry, and `protocol_mapping.rs`
   converts results into MCP content blocks. An unknown tool name is the only protocol-level
   error; every other failure is an error result the client can show.
5. `deskwright`'s `main.rs` serves the handler over stdio.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| **Registry** of tool types | Adding a tool doesn't edit a `match` (Open/Closed); one file per tool (Single Responsibility) | `rmcp`'s `#[tool_router]` macros on one large impl block |
| `TypedTool` blanket impl (a composition-based Template Method) | Argument parsing and schema generation are written once for all tools | Repeating parse-or-return code in every tool |
| **Adapter**: `DeskwrightMcpServer` + `protocol_mapping.rs` | Tools use the crate's own `ToolOutput`/`ToolSpec`; only two files know `rmcp` types | Tools returning `rmcp` types directly |
| Blocking work on tokio's blocking pool | Accessibility calls can take seconds and must not stall the protocol | Running ports on async worker threads |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Interface | `crates/deskwright-mcp/src/tool.rs` | `Tool`, `ToolSpec`, `ToolOutput`, `ToolDispatcher`, `ToolContext` |
| Interface | `crates/deskwright-mcp/src/typed_tool.rs`, `tool_arguments.rs` | Typed tools, parsing, schema generation |
| Interface | `crates/deskwright-mcp/src/tool_registry.rs` | Registry and dispatch |
| Interface | `crates/deskwright-mcp/src/server.rs`, `protocol_mapping.rs` | `rmcp` adapter |
| Interface | `crates/deskwright-mcp/src/blocking.rs` | Blocking-pool helper |
| Composition | `crates/deskwright/src/main.rs`, `composition.rs` | Registration and serving |

## Inputs and outputs

Standard MCP over stdio: `initialize`, `tools/list`, `tools/call`. Logs go to stderr; set
`RUST_LOG=debug` for more detail.

## How to extend

To add a tool:
1. Add any needed port or service in `deskwright-core` (with tests against `FakeDesktop`).
2. Create `crates/deskwright-mcp/src/tools/<name>_tool.rs` with an `Arguments` struct and a
   `TypedTool` impl, and export it from `tools.rs`.
3. Register it in `crates/deskwright/src/composition.rs` and update the tool list in the
   composition test.
4. Add a feature doc in `docs/features/` and a row in `docs/features/README.md`.

## Testing

`crates/deskwright-mcp/tests/tool_registry_tests.rs` covers duplicate names, lookup, sorting,
schemas, read-only hints, unknown tools and invalid arguments. The other `*_tests.rs` files
test each tool through the registry, using the shared fixture in `tests/support/mod.rs`. Unit
tests cover `protocol_mapping.rs`, the server's metadata, and the blocking helper's
panic-to-error conversion. `crates/deskwright/tests/stdio_protocol_smoke_test.rs` starts
the real binary, completes the handshake, lists and calls tools, and checks clean shutdown.
See [docs/TESTING.md](../TESTING.md) for the full layout and the check command.

## Limitations and future work

Only the stdio transport is wired up. `rmcp` also supports streamable HTTP if remote use is
ever needed.
