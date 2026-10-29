# Batch Execution

> Runs a whole flow, up to 50 tool calls, in order in one request, like a Playwright script.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Multi-step flows like "click the field, type, click Send" need several round trips. A batch
does them in one request and reports each step.

## How it works

1. `BatchTool` validates the list: not empty, at most `MAX_BATCH_ACTIONS` (50), and no nested
   `desk_batch`.
2. Each `BatchAction { tool, params }` is dispatched through the `ToolDispatcher` in the
   `ToolContext`, which is the same registry the server uses. Batched calls therefore get the
   same validation, logging and error handling as direct calls.
3. Output blocks of each action are collected under a `[index] tool ok|failed:` header.
4. By default the batch stops at the first failure, because later steps usually depend on
   earlier ones. `continueOnError: true` runs every step anyway.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| **Command**: actions are data executed by the batch | Calls can be validated as a whole before any of them runs | Hard-coding multi-step tools |
| Dispatch through `ToolContext`, not a registry reference held by the tool | Avoids a registry ↔ batch reference cycle; works with any dispatcher | Giving the batch an `Arc<ToolRegistry>` after construction |
| Stop on first failure by default | Prevents typing into, or clicking, the wrong element after a failed step | Always continuing, as in the original |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Interface | `crates/deskwright-mcp/src/tools/batch_tool.rs` | `desk_batch` |
| Interface | `crates/deskwright-mcp/src/tool.rs` | `ToolDispatcher`, `ToolContext` |
| Interface | `crates/deskwright-mcp/src/tool_registry.rs` | The dispatcher implementation |

## Inputs and outputs

Arguments: `actions: [{ tool, params? }]`, `continueOnError?`. Result: the combined blocks of all
executed actions; `isError` is true if any action failed.

Combined with [locators](locators.md), a batch needs no snapshot at all: each step finds and
waits for its own element, so an AI client sends the whole plan once and the server runs it at
native speed.

## How to extend

Batches pick up new tools automatically, since actions are dispatched by name.

## Testing

`crates/deskwright-mcp/tests/batch_tool_tests.rs` covers running in order, stopping at the first
failure, continuing when asked, the all-success case, and rejecting empty, oversized and
nested batches.

## Limitations and future work

Actions can't use earlier results (for example, refs from a snapshot in the same batch) except
by knowing them in advance.
