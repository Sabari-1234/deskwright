# Element Search

> Finds elements by role, title and/or value anywhere in an app's tree.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

When the client knows what it's looking for ("the Save button"), a search returns just
the matches instead of a whole tree, and the returned refs can be clicked immediately.

## How it works

1. `FindTool` builds an `ElementQuery` from `role`, `title` and `value`. At least one is
   required; blanks are ignored.
2. `ElementSearchService::find` takes a full-depth snapshot through
   `SnapshotService::capture_until_matches`, which stops walking the tree once it has found one
   match more than the limit; that extra match tells it more exist without reading the rest.
   The refs it returns are registered and usable.
3. It flattens the tree with the depth-first iterator and keeps the nodes the query matches,
   returning them as `ElementSummary` values (without children).

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| `title` also matches descriptions | WhatsApp labels every chat and button only in its description (found in live testing) | A separate `description` criterion clients would have to know about |
| Matching rules live in the `ElementQuery` value type | Case-insensitive matching and `AXButton`/`button` role normalization are domain rules, testable without any OS | Filtering inside the tool |
| Search reuses `SnapshotService` | One traversal implementation; results carry valid refs | A separate tree walk only for search |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/element_query.rs` | Query validation and matching |
| Application | `crates/deskwright-core/src/services/element_search_service.rs` | Snapshot + filter |
| Interface | `crates/deskwright-mcp/src/tools/find_tool.rs` | `desk_find` |

## Inputs and outputs

Arguments: `role?`, `title?` (substring of the title *or* the description, since apps such as
WhatsApp and most web content label elements only through descriptions), `value?` (substring), `app?`, `limit?` (1–200,
default 25), `format?`. Result: one line per match (`[e2] Button "Send"`), followed by a note
when matches were left out; `format: "json"` returns a JSON array instead. Errors: no criterion,
invalid limit.

## How to extend

Add a criterion, such as `description`, by adding a field and its check to `ElementQuery`, then
an argument to `FindArguments`.

## Testing

`element_query.rs` tests validation, role normalization and case-insensitive matching.
`element_search_service.rs` tests matches across nested elements.
`crates/deskwright-mcp/tests/element_tools_tests.rs` checks the tool's JSON result and the
"no criteria" error.

## Limitations and future work

Each search replaces the current refs, like a snapshot does.
