# UI Snapshot

> Reads an app's accessibility element tree and gives every element a short ref
> (`e1`, `e2`, …) that the other tools use to act on it.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

The snapshot is how a client "sees" an app: roles, titles, values, enabled/focused state and
on-screen bounds for each element, as structured JSON rather than pixels. It's the starting
point for clicking, typing, reading and element screenshots.

## How it works

1. `SnapshotTool` parses `app` and `maxDepth` (validated into `TreeDepth`, 1–50, default 10).
2. `SnapshotService::capture` checks `AccessibilityPermission`, resolves the app with
   `AppResolver`, and asks `ElementTreeReader` for its windows (or its children if it has none).
   Open pop-up and context menus come first: AppKit attaches them to the app rather than to
   their window, so without this their items could not be found, clicked or waited for.
3. `TreeWalker` walks depth-first, reading each element's attributes *and children* in one
   batched request (`ElementTreeReader::read_with_children`, implemented with
   `AXUIElementCopyMultipleAttributeValues`), one IPC round trip per element and numbering elements in visiting order,
   starting from the registry's next unused number.
   Elements that vanish mid-walk are skipped and counted. The walk stops at
   `MAX_SNAPSHOT_ELEMENTS` (5,000) and marks the snapshot as truncated.
4. The ref → element map replaces the `ElementRegistry` contents in one step. Numbers keep
   increasing across snapshots (a first snapshot hands out `e1`…`e69`, the next one `e70`…), so
   a ref from an earlier snapshot never resolves, rather than silently naming a different
   element. Captures run one at a time so two can't hand out the same numbers.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| **Composite** `ElementNode` with a depth-first **Iterator** | Leaves and containers share one type; search and flattening reuse the iterator | Separate flat list plus parent IDs |
| Opaque `NativeElement` handle | The core stores platform elements without knowing their type, which keeps OS code out of it | Generic `Handle` parameter on every port and service |
| Registry replaced as a whole per snapshot | Refs are always consistent with one snapshot | Accumulating refs across snapshots |
| One request per element for attributes and children | Each request is an IPC round trip to the app; batching children in cut finds from 50–73 ms to 5–15 ms and snapshots by about a third (measured on Finder, Chrome, WhatsApp) | Separate attribute and children requests |
| Ref numbers increase across snapshots | When every snapshot restarted at `e1`, a stale ref silently named whatever element now had that number (found in live testing) | Restarting at `e1`, or random IDs (longer, harder to read) |
| Element cap and unreadable-element count | Huge apps can't flood the client, and partial reads are reported, not hidden | Failing the whole snapshot on one bad element |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/element.rs` | `ElementRef`, `ElementAttributes`, `ElementSummary`, `ElementNode` |
| Domain | `crates/deskwright-core/src/model/snapshot.rs`, `model/tree_depth.rs` | Snapshot result, depth limits |
| Domain | `crates/deskwright-core/src/element_registry.rs` | Ref → `NativeElement` map |
| Application | `crates/deskwright-core/src/services/snapshot_service.rs`, `services/tree_walker.rs` | Capture use case and traversal |
| Adapter | `crates/deskwright-macos/src/accessibility/ax_tree_reader.rs`, `ax_element.rs`, `ax_value_conversion.rs` | Accessibility API reading |
| Interface | `crates/deskwright-mcp/src/tools/snapshot_tool.rs` | `desk_snapshot` |

## Inputs and outputs

Arguments: `app?` (name or bundle ID), `maxDepth?` (1–50), `format?`. By default the result is
a compact outline (`element_outline.rs`): a header line with the app and element count, then one
indented line per element such as `[e3] Button "Send" value="…" (disabled)`. Untitled layout
containers (groups, scroll areas, split groups) get no line of their own; their children move
up a level and their refs stay valid. With `format: "json"` the result is JSON
`{ app, elements: [ElementNode], elementCount, unreadableElementCount, isTruncated }`, where
each node has `ref`, `role`, optional `title`/`value`/`description`, `isEnabled`, `isFocused`,
optional `bounds`, and `children`. Errors: missing permission, app not found, invalid depth.

## How to extend

- To read another attribute, add it to `ELEMENT_ATTRIBUTE_NAMES` and `attributes_from_values`
  in `ax_tree_reader.rs`, and add a field to `ElementAttributes`.
- For a new OS, implement `ElementTreeReader`. The traversal, refs and limits are reused.

## Testing

`snapshot_service.rs` covers ref order and registration, depth limiting, skipped elements, and
the permission check; `tree_depth.rs` and `element.rs` cover depth limits, ref parsing and
depth-first order. `ax_tree_reader.rs` and `ax_value_conversion.rs` test attribute parsing with
real Core Foundation values. `crates/deskwright-mcp/tests/element_tools_tests.rs` checks the JSON
output, depth validation, and that snapshot refs work with `desk_click`.
`crates/deskwright-macos/tests/accessibility_integration_test.rs` reads the real Finder when
Accessibility permission is granted.

## Limitations and future work

- Refs are global: a snapshot of one app invalidates refs from another.
- Ref numbers grow for as long as the server runs; restarting the server starts again at `e1`.
- Before reading a tree, `AxTreeReader` sets `AXManualAccessibility`, which makes Electron apps
  expose their web content (not yet verified live). Chrome ignored both that and
  `AXEnhancedUserInterface` in live testing, so its pages show only the browser UI unless web
  accessibility is turned on in `chrome://accessibility`.
- Very large trees are truncated at 5,000 elements; there is no paging yet.
