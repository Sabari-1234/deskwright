# Deskwright — Architecture

> An MCP server (over stdio), a command-line flow runner and a Rust library that read and
> drive desktop apps through the OS accessibility tree instead of screenshots, the way
> Playwright drives browsers. macOS is supported today; the design keeps every OS-specific
> detail behind small port traits, so a Windows (UI Automation) or Linux (AT-SPI) backend is
> one new adapter crate, without touching the core, the tools or the runner.

**Last updated:** 2026-10-29

## Overview diagram

```
crates/deskwright   (composition root: library facade `Deskwright`, CLI, wiring, stdio)
        │                     │
        │               crates/deskwright-runner (delivery: flows, timings, reporters)
        │                     │
crates/deskwright-mcp       (delivery: one type per MCP tool, tool registry, rmcp adapter)
        │
crates/deskwright-core      (domain models, port traits, use-case services)
        ▲
        │ implement ports
crates/deskwright-macos     (macOS Accessibility, running apps, osascript)
crates/deskwright-portable  (enigo input, xcap screen capture: any OS)
```

## Layers and modules

| Crate | Layer | Responsibility | May depend on |
|---|---|---|---|
| `deskwright-core` | Domain + application | Value types (`model/`), port traits (`ports/`), the element registry, use-case services (`services/`), in-memory fakes (`testing/`, test-only) | `serde`, `thiserror` only |
| `deskwright-macos` | Infrastructure | Accessibility tree reader, element actions, menu bar locator, permission check, running-app directory, AppleScript runner | `deskwright-core` |
| `deskwright-portable` | Infrastructure | Synthetic keyboard/mouse input (`enigo`) and screen capture (`xcap`) | `deskwright-core` |
| `deskwright-mcp` | Interface | `Tool`/`TypedTool` traits, one file per tool, `ToolRegistry`, `DeskwrightMcpServer` (rmcp adapter) | `deskwright-core` |
| `deskwright-runner` | Interface | Flow files, the `FlowRunner` (runs steps through any `ToolDispatcher`, times them, takes trace screenshots), reporters (text, JSON, HTML trace) | `deskwright-mcp` |
| `deskwright` | Composition root | Library facade `Deskwright` (`src/lib.rs`), configuration, platform adapter factory, wiring; the binary's CLI (`src/cli.rs`, `src/commands/`) and stdio serving | all crates |

## Dependency rule

Dependencies point inward to `deskwright-core`. Separate crates make Cargo enforce this:
`deskwright-core` doesn't depend on any other workspace crate, adapters and tools only depend on
`deskwright-core`, and only the binary sees every crate. `deskwright-macos` compiles to an empty crate on
other operating systems, and the binary depends on it only on macOS.

## Ports (interfaces)

| Port | Purpose | Production adapter | Test fake |
|---|---|---|---|
| `AccessibilityPermission` | Is UI access allowed? | `AxAccessibilityPermission` | `FakeDesktop` |
| `AppDirectory` | Running apps, frontmost app | `WorkspaceAppDirectory` | `FakeDesktop` |
| `ElementTreeReader` | Top-level elements, attributes, children | `AxTreeReader` | `FakeDesktop` |
| `ElementActions` | Press, focus, set value, show menu, scroll into view | `AxElementActions` | `FakeDesktop` |
| `WindowController` | Minimize, restore, move, resize, close, raise windows | `AxWindowController` | `FakeDesktop` |
| `MenuBarLocator` | An app's menu bar | `AxMenuBarLocator` | `FakeDesktop` |
| `HitTester` | Which app is topmost at a screen point | `AxHitTester` | `FakeDesktop` |
| `AppLauncher` | Launch or activate an app, optionally with an address | `OpenCommandAppLauncher` | `FakeDesktop` |
| `AppTerminator` | Ask an app to quit, or force it | `RunningAppTerminator` | `FakeDesktop` |
| `Clipboard` | Read and write the clipboard's text | `PasteboardClipboard` | `FakeDesktop` |
| `InputSynthesizer` | Synthetic typing, clicks, double/right clicks, pointer moves, drags and key chords | `EnigoInputSynthesizer` | `FakeDesktop` |
| `ScreenCapturer` | Display or region PNG capture | `XcapScreenCapturer` | `FakeDesktop` |
| `ScriptRunner` | AppleScript/JXA execution | `OsaScriptRunner` | `FakeDesktop` |

Platform UI elements cross the ports as the opaque `NativeElement` handle; only the adapter
that created one can look inside it.

## Composition root

- `crates/deskwright/src/platform.rs` is a **Factory** that builds the current OS's adapters
  into a `PlatformAdapters` value. It's the only code that names concrete adapters.
- `crates/deskwright/src/composition.rs` builds the services from those adapters, sets the
  strategy order for click and text entry, and registers the tools. It's the only code that
  constructs services or tools.
- `crates/deskwright/src/lib.rs` is the **Facade** `Deskwright`: `Deskwright::launch` builds the
  adapters and tools, `call(tool, json)` runs one, and `dispatcher()` hands the registry to the
  MCP server and the flow runner. The binary (`src/main.rs`) only parses the CLI and calls one
  command module.
- `crates/deskwright/src/configuration.rs` reads flags and environment variables:
  `--enable-script` / `DESKWRIGHT_ENABLE_SCRIPT` (default off), `--script-timeout-seconds` (30),
  `--accessibility-timeout-seconds` (3). `RUST_LOG` sets log verbosity (default `info`).

## Cross-cutting concerns

- **Testing:** every layer is unit-tested; the core runs against the in-memory `FakeDesktop`,
  and adapter logic is split out of thin OS-call wrappers so it can be tested without
  permissions. `./scripts/check.sh` runs formatting, lint, all tests and the coverage gates
  (≥ 95% project, ≥ 98% core), and must pass before and after every change. See
  [`docs/TESTING.md`](TESTING.md).
- **Errors:** one `DeskwrightError` enum in the core, with messages written for the end user.
  Tools turn failures into MCP error *results* the client can show. Only an unknown tool
  name becomes a protocol error.
- **Concurrency:** ports and services are synchronous. Tools run them on tokio's blocking pool
  (`deskwright-mcp/src/blocking.rs`) so slow Accessibility calls never stall the protocol loop.
  The element registry is a mutex-guarded map replaced as a whole on each snapshot.
- **Unsafe code:** confined to `deskwright-macos`'s system-call modules (`src/accessibility/`, mostly
  `ax_element.rs`, and `src/process_list.rs`), and every block has a `// SAFETY:` explanation.
- **Logging:** `tracing` to stderr, because stdout carries the protocol. Each tool call logs its
  name, success and duration, never its arguments, which can contain passwords or scripts.
- **Security:** the script tool is opt-in. The type tool never echoes the typed text. Tools
  carry read-only/destructive annotations so clients can ask before risky calls.

## Key decisions

| Date | Decision | Reason |
|---|---|---|
| 2026-10-28 | Rust workspace with one crate per layer | Cargo enforces the dependency rule; a Windows port only adds an adapter crate |
| 2026-10-28 | Official `rmcp` SDK instead of a hand-written JSON-RPC server | Correct protocol framing and version negotiation without code to maintain |
| 2026-10-28 | Strategy + Chain of Responsibility for click and text entry | Replaces the original's unused per-app "smart router" with fallbacks that actually run |
| 2026-10-28 | `enigo` and `xcap` in a separate `deskwright-portable` crate | Both work on every OS, so future backends reuse them |
| 2026-10-28 | Script tool disabled unless `--enable-script` is set | Running AppleScript is equivalent to running any shell command as the user |
| 2026-10-28 | Frontmost app read live through Accessibility | `NSWorkspace` active-app state goes stale without a main run loop |
| 2026-10-29 | One check script with `cargo llvm-cov` coverage gates | Any person or agent can confirm in one command that a change broke nothing |
| 2026-10-29 | App list built from the kernel process list | Live testing showed `NSWorkspace` misses apps launched after startup |
| 2026-10-29 | `HitTester` port guards pointer clicks | Live testing showed clicks on covered elements landing in the window on top |
| 2026-10-29 | Compact text outline as the default output of snapshot, find and wait | JSON with bounds was many times longer, which made AI clients slow |
| 2026-10-29 | Locators with auto-wait on click, type and read | One call per action and whole flows in one batch, like Playwright |
| 2026-10-29 | One IPC round trip per element; searches stop early | Finds dropped to 5–15 ms on real apps |
| 2026-10-29 | Element ref numbers increase across snapshots | A stale ref must fail, not name a different element |
| 2026-10-29 | Key presses refused unless the target app is frontmost | Same safety rule as pointer clicks |
| 2026-10-29 | Click kinds (primary, double, secondary) each get a strategy chain; `PointerTarget` shared by every pointer gesture | Playwright-style gestures without duplicating the cover check |
| 2026-10-29 | `MenuNavigator` shared by menu-bar navigation and pop-up option selection | Both menus have the same shape |
| 2026-10-29 | Scoped locators (`within`) resolved during the tree walk | Playwright's `locator.locator()`; keeps early stopping |
| 2026-10-29 | Waits poll on a backoff schedule; no `AXObserver` tree cache | ~20 ms reaction without the stale-state risk of a notification-invalidated cache |
| 2026-10-29 | Flow runner in its own crate, over `ToolDispatcher` | Flows, the CLI and the MCP batch tool share one dispatch path; the runner is testable without the OS |
| 2026-10-29 | Library facade `Deskwright` in the composition crate | Rust callers get Playwright-style library use without seeing the wiring |
| 2026-10-29 | Expectations as `ElementCondition` strategies polled by one service | Playwright-style assertions; a new condition is a new type |
| 2026-10-29 | Separate `WindowController`, `AppTerminator` and `Clipboard` ports | Each is its own capability; no adapter implements methods it doesn't need |
| 2026-10-29 | Display clipping and PNG encoding moved out of the `xcap` adapter | Keeps permission-gated wrappers thin, so their logic can be unit-tested |

## Feature index

See [`docs/features/README.md`](features/README.md).
