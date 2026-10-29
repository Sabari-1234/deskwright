# Element Reading

> Reads the live state of one element by ref, without a new snapshot.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

After typing or clicking, a client often needs to check one value (a text field's content, a
checkbox state). Reading one element is much cheaper than a new snapshot and keeps the
current refs valid.

## How it works

1. `ReadTool` parses `ref` into an `ElementRef`.
2. `ElementReadService::read` resolves it in the `ElementRegistry` and asks
   `ElementTreeReader::attributes` for the element's current attributes.
3. The result is returned as an `ElementSummary` in JSON.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| Reads the live element, not the snapshot copy | The point is to see changes since the snapshot | Returning cached snapshot data |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Application | `crates/deskwright-core/src/services/element_read_service.rs` | Resolve + read |
| Interface | `crates/deskwright-mcp/src/tools/read_tool.rs` | `desk_read` |

## Inputs and outputs

Argument: `ref`. Result: `{ ref, role, title?, value?, description?, isEnabled, isFocused, bounds? }`.
Errors: malformed ref, unknown ref (take a new snapshot), element no longer exists.

## How to extend

New attributes appear here automatically once `ElementAttributes` gains them (see the
[UI snapshot](ui-snapshot.md) doc).

## Testing

`element_read_service.rs` tests reading a changed value and rejecting unknown refs.
`crates/deskwright-mcp/tests/element_tools_tests.rs` checks that `desk_read` returns a value written
by `desk_type`, and rejects malformed refs.

## Limitations and future work

None known.
