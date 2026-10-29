# App Discovery

> Lists the running apps and decides which app a request is about.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Clients need to know which apps are open before snapshotting one. Every app-scoped tool
(`desk_snapshot`, `desk_find`, `desk_menu`) accepts an optional app name or bundle ID, and
targets the frontmost app when none is given.

## How it works

1. `ListAppsTool` calls `AppDirectory::running_apps` on the blocking pool and returns JSON.
2. `WorkspaceAppDirectory` reads every process ID live from the kernel (`proc_listallpids`, in
   `process_list.rs`), asks `NSRunningApplication` about each one, and keeps the apps whose
   activation policy is *regular* (apps that appear in the Dock).
3. Each freshly created `NSRunningApplication` also reports whether it is the active app.
4. Other tools turn their `app` argument into an `AppSelector` (blank → frontmost), and
   `AppResolver` matches it against the directory by display name or bundle ID, ignoring case.

App names are matched ignoring case and invisible direction marks: WhatsApp reports its
name as `"\u{200e}WhatsApp"`, which made `"app": "WhatsApp"` fail until live testing found it.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| `AppSelector` value type built at the tool boundary | Converts the "blank means frontmost" rule once, so services get a clear intent | Passing `Option<String>` through every layer |
| `ListAppsTool` uses the `AppDirectory` port directly | Listing is the whole use case; a wrapper service would add nothing | A pass-through `ListAppsService` |
| Frontmost app from fresh `NSRunningApplication` objects | Accurate without a run loop; the Accessibility focused-app query returned nothing in live testing | `AXFocusedApplication` on the system-wide element |
| App list built from the kernel's process list on every call | `NSWorkspace.runningApplications` only refreshes on a main run loop, so apps launched after startup were missing (found in live testing) | Running a main run loop on a dedicated thread |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/app.rs` | `AppInfo`, `ProcessId`, `AppSelector` |
| Domain | `crates/deskwright-core/src/ports/app_directory.rs` | `AppDirectory` port |
| Application | `crates/deskwright-core/src/services/app_resolver.rs` | Resolves selectors to running apps |
| Adapter | `crates/deskwright-macos/src/workspace_app_directory.rs` | Running-app directory |
| Adapter | `crates/deskwright-macos/src/process_list.rs` | Live process IDs from the kernel |
| Interface | `crates/deskwright-mcp/src/tools/list_apps_tool.rs` | `desk_list_apps` |

## Inputs and outputs

`desk_list_apps` takes no arguments and returns an array of
`{ name, bundleId?, processId, isActive }`. App-scoped tools fail with
`no running app matches '<name>'` or `there is no frontmost app`.

## How to extend

To support another OS, implement `AppDirectory` in that OS's adapter crate and return it
from `platform::native_adapters`.

## Testing

`app_resolver.rs` tests name and bundle-ID matching and both "not found" errors against
`FakeDesktop`. `workspace_app_directory.rs` lists the real running apps (no permission
needed) and checks that the frontmost app is among them. `process_list.rs` checks that a
process started after an earlier listing appears in the next one.
`crates/deskwright-mcp/tests/app_tools_tests.rs` checks the `desk_list_apps` JSON, and the stdio
smoke test calls it through the real binary.

## Limitations and future work

- Apps that don't appear in the Dock (menu-bar utilities, agents) aren't listed.
- Each listing queries every process, which costs a few milliseconds (about 10 ms measured).
