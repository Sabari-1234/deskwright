# App Launching

> Launches an app, or brings it to the front, and waits until it is frontmost.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Key presses, menus of background apps, and pointer clicks all need the target app in front.
`desk_launch_app` makes that one call instead of a terminal command, and can open an http(s)
address in the app (for example a search results page in a browser).

## How it works

1. `LaunchAppTool` validates the optional `url` into a `WebAddress` (http and https only).
2. `AppLaunchService::open` rejects a blank name, calls the `AppLauncher` port, then resolves
   the app through `AppResolver` and returns its `AppInfo`.
3. `OpenCommandAppLauncher` runs `/usr/bin/open -a <name>` (or `-b <bundle id>` for reverse-DNS
   IDs), optionally followed by the address, with arguments passed directly and no shell. It
   then polls `WorkspaceAppDirectory::frontmost_app` until the app is frontmost (10 s limit).

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| `AppLauncher` port; waiting lives in the adapter | The core stays deterministic; only the macOS adapter knows activation is asynchronous | Polling in the service with an injected clock |
| System `open` command | Launch Services handles launching, activation and URL hand-off in one call | `NSWorkspace.openApplication` with completion blocks |
| `WebAddress` allows only http/https | `file:` and custom app schemes can trigger local actions | Accepting any URL |
| Frontmost check through the app directory | The Accessibility focused-app query returned nothing in this server (found in live testing), which made every launch wait the full 10 s | `AXFocusedApplication` on the system-wide element |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/web_address.rs` | Validated http(s) address |
| Domain | `crates/deskwright-core/src/ports/app_launcher.rs` | `AppLauncher` port |
| Application | `crates/deskwright-core/src/services/app_launch_service.rs` | Launch and report the app |
| Domain | `crates/deskwright-core/src/polling.rs` | Shared `wait_until` helper |
| Adapter | `crates/deskwright-macos/src/open_command_app_launcher.rs` | `open` command + frontmost wait |
| Interface | `crates/deskwright-mcp/src/tools/launch_app_tool.rs` | `desk_launch_app` |

## Inputs and outputs

Arguments: `app` (name or bundle ID), `url?`. Result: the app as JSON
(`name`, `bundleId`, `processId`, `isActive`). Errors: blank name, non-http(s) address,
`no installed app matches '<name>'`, or the app not becoming frontmost in time.

## How to extend

Another OS implements `AppLauncher` in its adapter crate and returns it from
`platform::native_adapters`.

## Testing

`app_launch_service.rs` (activate running app, launch new app with address, blank name, not
installed), `web_address.rs`, `polling.rs`, `open_command_app_launcher.rs` (argument building,
error mapping, a real `open` call for a missing app), and
`crates/deskwright-mcp/tests/launch_and_keys_tools_tests.rs`. Measured live: about 100–180 ms to
activate Finder, TextEdit or Chrome.

## Limitations and future work

- Only http and https addresses can be opened.
