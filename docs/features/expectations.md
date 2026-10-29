# Expectations (Auto-retrying Assertions)

> Asserts that located elements reach a state, retrying until they do, like Playwright's
> web-first `expect(locator).toBeVisible()` / `toHaveText()` / `toHaveCount()`.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Flows need to verify their effects ("the message appears in the chat", "Save is enabled",
"the dialog is gone") without guessing how long the app takes. A one-shot read is flaky; a
fixed sleep is slow. An expectation polls until the state holds and fails with what it saw.

## How it works

1. `desk_expect` takes a locator (refs are refused: a ref names one element from one
   snapshot, so it can't be re-searched), a `condition`, and `text` or `count` where needed.
2. The tool turns the condition into one `ElementCondition` (**Strategy**):
   `IsVisible`, `IsHidden`, `IsEnabled(bool)`, `IsChecked(bool)`, `IsFocused`,
   `HasText { text, is_exact }` (value, or title when there is no value) and `HasCount(n)`.
3. `ExpectationService::expect` searches every 150 ms until the condition holds or the
   locator's timeout (default 5 s, at most 30 s) passes. Each check builds an `Observation`:
   all matches plus the selected one (`nth`, default 0, or `last`). The search stops early
   after the selected match unless the condition needs every match (`HasCount`).
4. Success returns `Passed: <query> <condition> (<ref>).` Failure returns
   `expected <query> <condition> within <ms> ms, but <what was seen>`, for example
   `… to be disabled within 5000 ms, but e12 is enabled`.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| One type per condition behind `ElementCondition` | Adding an assertion is a new type, not a new branch; each is tested alone | A `match` on a condition enum inside the service |
| Conditions in the domain model | They are pure rules over element data with no I/O | Placing them in the service |
| Locators only | Expectations re-search the live UI each time, which a ref can't do | Re-reading a ref's element |
| Failure says what was seen | The client can correct the flow without another snapshot | A bare "timed out" |
| Shared with `desk_wait_for { state: "hidden" }` | One polling implementation for both | A second polling loop |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/element_condition.rs` | `ElementCondition`, `Observation`, the conditions |
| Domain | `crates/deskwright-core/src/model/element.rs` | `is_checked`, `is_visible` |
| Domain | `crates/deskwright-core/src/error.rs` | `ExpectationFailed` |
| Application | `crates/deskwright-core/src/services/expectation_service.rs` | Polling and selection |
| Interface | `crates/deskwright-mcp/src/element_target_arguments.rs` | `to_required_locator` |
| Interface | `crates/deskwright-mcp/src/tools/expect_tool.rs` | `desk_expect` |

## Inputs and outputs

```json
{ "tool": "desk_expect", "params": { "app": "WhatsApp", "role": "statictext", "title": "hi", "last": true, "condition": "visible" } }
{ "tool": "desk_expect", "params": { "role": "button", "title": "Save", "condition": "enabled" } }
{ "tool": "desk_expect", "params": { "role": "textfield", "title": "To", "condition": "containsText", "text": "@example.com" } }
{ "tool": "desk_expect", "params": { "role": "row", "condition": "count", "count": 3 } }
```

## How to extend

Implement `ElementCondition` for a new type in `element_condition.rs` (with tests), and add
an `ExpectedCondition` value that builds it in `expect_tool.rs`.

## Testing

`element_condition.rs` tests every condition's met and unmet cases and descriptions.
`expectation_service.rs` tests selection (`last`), counting, failure messages, a missing
element counting as hidden, and search failures. `crates/deskwright-mcp/tests/expect_tool_tests.rs`
tests the tool, its argument errors and the hidden wait.

## Limitations and future work

- Visibility means "has a non-empty frame"; an element scrolled out of its container but still
  in the tree counts as visible.
- There are no negated forms (`not.toHaveText`) apart from the paired conditions.
