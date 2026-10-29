# Window Management

> Lists an app's windows and minimizes, restores, closes, raises, moves or resizes one,
> like Playwright's `page.bringToFront()`, `setViewportSize()` and `page.close()`.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Flows often need a known window layout (for reproducible screenshots and pointer
coordinates), a covered window brought forward before a pointer click, or a stray document
window closed. One tool covers all of these.

## How it works

1. `WindowTool` parses `action` into a `WindowCommand` (or "list"), validating geometry at
   the boundary: `move` needs finite `x` and `y`, `resize` needs a positive `width` and `height`.
2. `WindowService` checks the Accessibility permission, resolves the app, and reads its
   top-level elements, keeping only `AXWindow`s (sheets and drawers are skipped). The order is
   the app's own: frontmost first.
3. A `WindowSelector` picks one window: the `index`-th (default 0) of the windows whose title
   contains `title` (ignoring case).
4. The command runs through the `WindowController` port:
   - minimize and restore set `AXMinimized`, then read it back for up to 1 s: apps accept the
     request and ignore it with Stage Manager on (found in live testing), so the tool fails
     instead of reporting a change that didn't happen;
   - move and resize set `AXPosition` / `AXSize` with `AXValue`-wrapped points and sizes;
   - close presses the window's `AXCloseButton`, so the app's own close handling (save
     prompts) runs;
   - raise restores the window if it is minimized, performs `AXRaise`, then activates the app through
     `AppLauncher`, because raising alone only reorders windows inside a background app.
5. The window is read again and reported, so the result shows its new bounds and state. A
   closed window is reported as it was.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| One `desk_window` tool with an `action` | Seven small tools would crowd the client's tool list | One tool per action |
| A new `WindowController` port | Window commands are a separate capability from element actions (ISP) | Adding them to `ElementActions` |
| `WindowCommand` (**Command**) as data | Validated once at the edge, then run by one `match` in the service | Separate service methods per action |
| Close presses the close button | Same behavior as the user clicking it, including save prompts | Destroying the window (not possible through AX) |
| Raise also activates the app | Otherwise the window stays behind the frontmost app | Requiring a separate `desk_launch_app` |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/window.rs` | `WindowSummary`, `WindowSelector`, `WindowCommand` |
| Domain | `crates/deskwright-core/src/ports/window_controller.rs` | The port |
| Application | `crates/deskwright-core/src/services/window_service.rs` | Listing, selecting, running commands |
| Adapter | `crates/deskwright-macos/src/accessibility/ax_window_controller.rs` | AX implementation |
| Adapter | `crates/deskwright-macos/src/accessibility/ax_value_conversion.rs` | Point and size `AXValue`s |
| Interface | `crates/deskwright-mcp/src/tools/window_tool.rs` | `desk_window` |

## Inputs and outputs

- `desk_window { action: "list", app: "Pages" }` →
  `[0] "Report.pages" at 10,20 size 800x600` and one line per further window, with
  `(minimized)` where it applies.
- `desk_window { action: "resize", title: "report", width: 1200, height: 900 }` →
  `Resized [0] "Report.pages" at 10,20 size 1200x900.`
- No match: `could not find the window: Pages has no window titled "budget"`.

## How to extend

A new window action (for example full screen) is a `WindowCommand` variant, a
`WindowController` method, and a `WindowAction` value in the tool.

## Testing

`window.rs` tests title matching, descriptions and geometry validation. `window_service.rs`
tests listing, minimize and restore, move and resize, raise, close, the no-match error and the
permission check. `crates/deskwright-mcp/tests/window_and_app_tools_tests.rs` tests the tool output
and argument errors. The AX adapter changes real windows, so it is excluded from the coverage
gate (see `docs/TESTING.md`).

## Limitations and future work

- Apps that refuse `AXSize` (fixed-size windows) report an error from the adapter.
- Full screen and moving windows between displays or Spaces are not supported.
