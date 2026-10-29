# App Quitting

> Quits an app as its Quit menu item would, and waits until it has really exited.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Tests and flows need to start from a clean app state, the way Playwright closes its browser.
Asking an app to quit is asynchronous, and the app may refuse (a save prompt), so the tool
waits and says what happened.

## How it works

1. `desk_quit_app { app, force? }` requires an app name: quitting never defaults to the
   frontmost app.
2. `AppQuitService` resolves the app, then asks the `AppTerminator` port to quit it. The macOS
   adapter calls `NSRunningApplication.terminate` (the app's own quit handling) or, with
   `force`, `forceTerminate`.
3. It polls the `AppDirectory` every 50 ms, for up to 5 s, until the process is gone. If the
   app is still running, the error says it is probably asking to save changes.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| App name required | A mistaken quit can lose work | Defaulting to the frontmost app like other tools |
| Wait for the process to exit | The next step (relaunching, for example) must not race the old process | Returning right after the request |
| `force` off by default, with a warning in the description | Forced quits discard unsaved work | Escalating to force automatically |
| A separate `AppTerminator` port | Quitting is unrelated to launching or listing (ISP) | Adding `quit` to `AppLauncher` |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/ports/app_terminator.rs` | The port |
| Application | `crates/deskwright-core/src/services/app_quit_service.rs` | Request, then wait for exit |
| Adapter | `crates/deskwright-macos/src/running_app_terminator.rs` | `NSRunningApplication` |
| Interface | `crates/deskwright-mcp/src/tools/quit_app_tool.rs` | `desk_quit_app` |

## Inputs and outputs

`desk_quit_app { app: "TextEdit" }` → `TextEdit has quit.`, or the error
`could not quit the app: TextEdit is still running; it may be asking to save changes …`.

## How to extend

Make the exit timeout configurable by passing it from `DeskwrightConfiguration` in the
composition root.

## Testing

`app_quit_service.rs` tests a normal quit, an app that ignores the request, a forced quit,
and the missing-name and unknown-app errors. `window_and_app_tools_tests.rs` tests the tool
messages. The adapter quits real apps, so it is excluded from the coverage gate.

## Limitations and future work

Answering the save prompt is left to the client (for example `desk_click` on "Don't Save").
