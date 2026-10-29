# Command Line and Library API

> One binary with four commands (`serve`, `tools`, `call`, `run`), and a Rust library facade
> (`deskwright::Deskwright`) for driving the desktop from code.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Playwright is usable from a test runner, from scripts and from code. Deskwright offers the
same three ways in: MCP for AI clients, a CLI for scripts and CI, and a Rust API.

## How it works

1. `src/main.rs` parses `Cli` (clap). The configuration flags (`--enable-script`, the
   timeouts) are global, so they work before or after the command. No command means `serve`,
   so existing MCP client configurations keep working.
2. It builds one `Deskwright` with `Deskwright::launch(&configuration)`, which creates the native
   adapters (`platform.rs`) and the tool registry (`composition.rs`).
3. It calls one module in `src/commands/`:
   - `serve` hands `deskwright.dispatcher()` to the rmcp server on stdio;
   - `tools` prints each tool's name and the first sentence of its description;
   - `call <tool> [json] [--image-dir dir]` runs one tool, prints its text (images are
     summarized, and saved as PNG files with `--image-dir`), and exits with 1 on failure;
   - `run` runs flows (see [flow runner](flow-runner.md)).
4. From Rust, `Deskwright::call(tool, serde_json::Value)` runs a tool and returns its
   `ToolOutput`; `tools()` lists them.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| One binary, `serve` by default | MCP configurations written for the server-only binary still work | Separate binaries |
| A **Facade** (`Deskwright`) over the registry | Library users get one type and never touch the wiring | Exposing `composition` directly |
| The library takes tool names and JSON | It matches the MCP tools exactly, so there is one contract to learn and keep stable | A second, typed API duplicating every tool |
| One module per command | Each command is small and testable; `main` stays a dispatcher | All commands in `main.rs` |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Composition | `crates/deskwright/src/lib.rs` | `Deskwright` facade |
| Composition | `crates/deskwright/src/cli.rs` | `Cli`, `CliCommand`, `ReportFormat` |
| Composition | `crates/deskwright/src/commands/*.rs` | `serve`, `tools`, `call`, `run` |
| Composition | `crates/deskwright/src/main.rs` | Parse, launch, dispatch, exit code |

## Inputs and outputs

See the README's "Command line" and "Rust library" sections.

## How to extend

A new command is a `CliCommand` variant, a module in `src/commands/`, and one arm in `main`.

## Testing

`cli.rs` tests parsing (default command, global flags after commands, limits). `lib.rs`
tests the facade against the fake desktop. `tools_command.rs` and `call_command.rs` test
their output and image saving. `crates/deskwright/tests/cli_tests.rs` runs the real binary.

## Limitations and future work

Bindings for other languages (Node, Python) can wrap `deskwright call` or the MCP server.
