# Locators

> Act on an element by describing it (role, title, value), the way Playwright locators work,
> instead of taking a snapshot and copying a ref.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

With refs, every action needs two calls: a snapshot or find to get the ref, then the action.
For an AI client each call is a full round trip. Locators resolve and act in one call, and wait
for the element to appear, so a whole flow (open, click, type, press Enter) can go in a single
`desk_batch` that runs on the server at native speed.

## How it works

1. Every element tool (`desk_click`, `desk_type`, `desk_read`, `desk_hover`,
   `desk_check`, `desk_select`, `desk_scroll_into_view`, and both ends of `desk_drag`)
   accepts either `ref` or locator fields, like Playwright's `getByRole`, `getByPlaceholder`
   and `getByTestId`:
   - `role` (`button` or `AXButton`), `title` (matches title or description), `value`,
     `placeholder` (the field's hint text) and `identifier` (the developer-set
     `AXIdentifier`, matched exactly);
   - `exact: true` for whole-string, case-sensitive matching instead of case-insensitive
     substrings (invisible direction marks are ignored either way);
   - `within: { role, title, … }` to match only inside a container, like Playwright's
     `locator.locator()`: for example `within: { "role": "window", "title": "Draft" }`, or a
     chat app's message pane so a contact's name in the sidebar is never picked;
   - `app`, then `nth` (0-based, default 0) or `last: true`, and `timeoutSeconds` (default 5).

   `ElementTargetArguments` parses them into an `ElementTarget`. Giving both a ref and a
   locator, or both `nth` and `last`, is an error.
2. `ElementLocatorService::resolve` passes refs through. For a locator it calls
   `ElementWaitService::wait_for_matches`, which polls an early-stopping search until at least
   `nth + 1` elements match, then returns the `nth` match's ref. For `last` it searches the
   whole tree (there is no early stop) and returns the final match.
3. The tool runs the action (click strategies, text entry, read) on that ref in the same blocking
   task, so resolving and acting happen back to back.

Scoping is handled during the tree walk: `ElementQuery::within` stores the scope, the walker
tracks whether it is inside a scope match, and `ElementQuery::matching_elements` applies the
same rule to the result, so early stopping still works for scoped searches.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| `ElementTarget` (Ref or Locator) in the core | Every action accepts both the same way; adding locator support to another tool is one field | Separate locator tools (`desk_click_by`, …) |
| Auto-wait built into resolution | Matches Playwright: no fixed delays before acting on appearing UI | Requiring `desk_wait_for` before every action |
| Locator search stops at `nth + 1` matches | Finding the first match usually reads only part of the tree | Full snapshot, then filter |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/element_target.rs` | `ElementTarget`, `Locator` |
| Application | `crates/deskwright-core/src/services/element_locator_service.rs` | Resolution with auto-wait |
| Application | `crates/deskwright-core/src/services/element_wait_service.rs` | `wait_for_matches` |
| Interface | `crates/deskwright-mcp/src/element_target_arguments.rs` | Shared `ref`/locator arguments |
| Interface | `crates/deskwright-mcp/src/timeout_argument.rs` | `timeoutSeconds` parsing |
| Interface | `crates/deskwright-mcp/src/tools/click_tool.rs`, `type_text_tool.rs`, `read_tool.rs` | Tools |

## Inputs and outputs

Example: `desk_click { "app": "WhatsApp", "role": "button", "title": "Send" }` returns
`Clicked e417 using accessibility press.` Errors: both ref and locator, no criteria, timeout
(`no element matching … appeared within … ms`), too few matches for `nth`.

## How to extend

Add locator support to another tool by flattening `ElementTargetArguments` into its arguments
and resolving with `ElementLocatorService` before the action.

## Testing

`element_locator_service.rs` (refs pass through, nth resolution, timeout),
`element_target_arguments.rs` (parsing rules), `crates/deskwright-mcp/tests/element_tools_tests.rs`
(click, type and read by locator; waiting and errors) and
`crates/deskwright-mcp/tests/batch_tool_tests.rs` (a whole locator flow in one batch).

## Limitations and future work

- Each wait poll re-reads the tree; caching trees and updating them from Accessibility
  notifications would make repeated locators nearly free.
