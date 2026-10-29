# Element Waiting

> Waits until elements matching a query appear, or until none is visible any more, instead of
> sleeping for a guessed time.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

After launching an app, opening a dialog or loading a page, a client needs the next element.
Fixed delays are either too short (flaky) or too long (slow). `desk_wait_for` returns as soon
as the element exists.

## How it works

1. `WaitForTool` validates the query, the `limit` and `timeoutSeconds` (default 5).
2. `ElementWaitService::wait_for` runs `ElementSearchService::find` repeatedly with the shared
   `polling::wait_until`, for at most 30 s. The pauses follow a `PollSchedule` set in the
   composition root: 20, 50 and 100 ms, then 150 ms from there on, like Playwright's auto-wait
   backoff. Fast UI changes are seen within about 20 ms, and slow apps aren't re-read in a
   tight loop. The last pause is cut short at the deadline. Search
   failures while waiting, such as an app that is still launching, are retried.
3. On success it returns the matches (with refs) as compact text. On timeout the error names the
   query and, if the last attempt failed, why.
4. With `state: "hidden"` the tool instead asks the [expectation service](expectations.md) to
   wait for the `IsHidden` condition: no match, or a first match with no visible area. It
   returns `No element matching … is visible.`, for example once a progress indicator or a
   dialog has gone away.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| Built on `ElementSearchService` | Same matching rules and refs as `desk_find` | A separate tree walk |
| Poll schedule injected, with backoff | Tests run with zero delay; production reacts within ~20 ms | A fixed 150 ms interval (the first version) |
| Polling instead of `AXObserver` notifications | Searches take 5–15 ms, so polling is already fast; notifications need a run-loop thread per app and are not sent by every app, and a missed one would leave a wait (or a cache) silently stale | An observer-driven tree cache |
| 30 s cap | A mistaken call can't hang a client for long | No cap |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Application | `crates/deskwright-core/src/services/element_wait_service.rs` | Polling search |
| Domain | `crates/deskwright-core/src/polling.rs` | `wait_until`, `PollSchedule` |
| Application | `crates/deskwright-core/src/services/expectation_service.rs` | The hidden state |
| Interface | `crates/deskwright-mcp/src/tools/wait_for_tool.rs` | `desk_wait_for` |

## Inputs and outputs

Arguments: `role?`, `title?`, `value?`, `app?`, `timeoutSeconds?`, `limit?`. Result: one line
per match. Errors: no criteria, invalid limit or timeout, timeout.

## How to extend

To wait for an element to disappear, add a mode that polls until `find` returns nothing.

## Testing

`element_wait_service.rs` (immediate match, timeout, last-failure reporting) and
`crates/deskwright-mcp/tests/element_tools_tests.rs`. Verified live: Chrome's address bar found in
190 ms after Cmd+T.

## Limitations and future work

- Each poll re-reads the app's tree, which is fine for normal apps but slower for very large ones.
