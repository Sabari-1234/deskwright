# Locator Suggestion (Pick Locator)

> Turns a snapshot ref into the most stable locator that finds that element, like
> Playwright's "pick locator" in codegen, so flows and batches can be written with locators
> instead of refs.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Refs (`e12`) change with every snapshot, so they can't be saved in a flow file. Writing a
good locator by hand means guessing which attributes are unique. `desk_locator` works it
out from the live UI.

## How it works

1. `desk_locator { ref, app? }` reads the element's attributes through the registry.
2. `SuggestedLocator::candidates_for` lists candidates, most stable first:
   - its `identifier` (set by developers, unaffected by language or content), except AppKit's
     generated `_NS:123` ones, which change between launches;
   - its role with its exact title (or description, when there is no title);
   - its role with its exact placeholder;
   - its role alone.
3. `LocatorSuggestionService` searches the app with each candidate and finds the target among
   the matches. Matches from a new snapshot are the same element when role, title,
   description, identifier and bounds are equal. The first candidate that matches only the
   target wins; otherwise the first candidate that finds it at all, with `nth` set.
4. The result is the locator as JSON, with `app` added when given, ready to paste into a
   flow step's `params`.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| Verify each candidate against the live app | A suggestion that matches the wrong element is worse than none | Choosing by attribute presence alone |
| Identifier first, then exact labels | Same priority as Playwright: test IDs, then role and accessible name | Value (changes as the user types) |
| Same-element check by identity attributes and bounds | Elements from different snapshots can't be compared directly through the ports | Comparing platform handles (needs a new port method) |
| Output in the tools' own argument shape | Paste-ready for flows, batches and `desk_expect` | A text description |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/suggested_locator.rs` | `SuggestedLocator`, candidate order |
| Application | `crates/deskwright-core/src/services/locator_suggestion_service.rs` | Verifying candidates |
| Interface | `crates/deskwright-mcp/src/tools/locator_tool.rs` | `desk_locator` |

## Inputs and outputs

`desk_locator { "ref": "e12", "app": "WhatsApp" }` →
`{"app":"WhatsApp","exact":true,"role":"button","title":"Send"}`

Checking candidates takes new snapshots, so `e12` and older refs are stale afterwards.

## How to extend

A new kind of candidate (for example role plus a `within` scope) is one more entry in
`SuggestedLocator::candidates_for`, placed by stability.

## Testing

`suggested_locator.rs` tests the candidate order, the description fallback, query building
and serialization. `locator_suggestion_service.rs` tests a unique title, `nth` for duplicates,
a unique identifier, and an element looked up in the wrong app.
`crates/deskwright-mcp/tests/expect_tool_tests.rs` tests the tool's JSON and ref validation.

## Limitations and future work

- A full recorder (Playwright's `codegen`: watching clicks and keys and writing the flow) is
  not built yet. It needs a system-wide event tap, which requires the Input Monitoring
  permission and a run-loop thread. This tool is the part such a recorder would use to name
  each clicked element.
- Bindings for Node and Python are not provided; they can wrap `deskwright call` or the
  MCP server.
