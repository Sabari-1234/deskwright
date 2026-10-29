# deskwright

**Playwright for desktop apps.** Drive native apps (Finder, Notes, Mail, WhatsApp, your own
app) through the operating system's accessibility API: find elements by role and label, click,
type, drag, assert, and record what happened, from an AI assistant (MCP), the command line, or
Rust.

[![CI](https://github.com/Sabari-1234/deskwright/actions/workflows/ci.yml/badge.svg)](https://github.com/Sabari-1234/deskwright/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

> **Platforms:** macOS today. The core, tools and runner are platform-independent; Windows
> (UI Automation) and Linux (AT-SPI) backends are planned, each as one adapter crate. See
> [Platforms](#platforms).

```bash
deskwright call desk_click '{ "app": "Notes", "role": "button", "title": "New Note" }'
```

- **Locators, not pixels.** Elements are found by `role`, `title`, `value`, `placeholder` or
  `identifier`, optionally `within` a container, and every action waits for its element to
  appear, like Playwright's `getByRole(...)`.
- **Real actions.** Click, double-click, right-click, hover, drag, type, press keys, choose
  menu items, check boxes, pick pop-up options, scroll, and manage windows, apps and the
  clipboard. Accessibility actions are used first, synthetic input as the fallback, and pointer
  input is refused when another app's window covers the target.
- **Auto-retrying assertions.** `desk_expect` waits until an element is visible, hidden,
  enabled, checked, focused, has some text, or a query matches a count.
- **Fast.** A search typically takes 5–15 ms, and waits re-check after 20 ms, so a 5-step flow
  runs in about half a second.
- **Flows, traces and CI.** Save steps as JSON, run them with `deskwright run`, and get
  per-step timings, JSON results, or an HTML trace with a screenshot after every step.
- **For AI assistants.** An MCP server with 23 tools (plus an opt-in script tool) and compact
  text output, so an assistant can drive your desktop in a few calls and tokens.

## Install

Requires macOS 13 or later (see [Platforms](#platforms)). Pick one:

```bash
# Homebrew
brew install sabari-1234/tap/deskwright

# Shell installer
curl -LsSf https://github.com/Sabari-1234/deskwright/releases/latest/download/deskwright-installer.sh | sh
```

An npm package (`npx -y deskwright`) is coming soon.

Prebuilt binaries for Apple Silicon and Intel are also attached to every
[GitHub Release](https://github.com/Sabari-1234/deskwright/releases).

<details>
<summary>Build from source</summary>

Requires Rust 1.85 or later.

```bash
git clone https://github.com/Sabari-1234/deskwright.git
cd deskwright
cargo build --release   # the binary is target/release/deskwright
```

</details>

### Permissions

macOS asks once for the app that runs deskwright (your terminal, or the MCP client):

- **Accessibility** (System Settings → Privacy & Security → Accessibility): required for
  everything that reads or acts on apps.
- **Screen Recording**: only for screenshots (`desk_screenshot` and `--trace`).

Restart deskwright after granting a permission.

## Use it from an AI assistant (MCP)

Add deskwright to your MCP client's configuration:

```json
{
  "mcpServers": {
    "deskwright": {
      "command": "deskwright"
    }
  }
}
```

If your MCP client doesn't find it, use the full path that `which deskwright` prints. Grant the Accessibility permission to the MCP client app (see below).

Then ask, for example: *"Open TextEdit, write a short poem, and save it in Downloads."* The
assistant uses tools such as `desk_snapshot`, `desk_click`, `desk_type` and `desk_expect`, and can
send a whole plan at once with `desk_batch`.

## Use it from the command line

```bash
deskwright tools                                  # list the tools
deskwright call desk_list_apps                     # run one tool
deskwright call desk_screenshot '{}' --image-dir shots
deskwright run flows/*.json                       # PASS/FAIL per flow, time per step
deskwright run note.json --trace trace.html       # HTML trace with a screenshot per step
deskwright run note.json --repeat 10              # benchmark a flow
deskwright run flows/*.json --reporter json       # machine-readable results for CI
```

A flow is a JSON list of tool calls:

```json
{
  "name": "Write a note",
  "steps": [
    { "tool": "desk_launch_app", "params": { "app": "TextEdit" } },
    { "tool": "desk_press_keys", "params": { "app": "TextEdit", "keys": "cmd+n" } },
    { "tool": "desk_type", "params": { "app": "TextEdit", "role": "textarea", "text": "Hello" }, "name": "type" },
    { "tool": "desk_expect", "params": { "app": "TextEdit", "role": "textarea", "condition": "containsText", "text": "Hello" } }
  ]
}
```

`run` exits with 1 if any flow failed, and `call` exits with 1 if the tool failed. Use
`desk_locator` to turn an element ref from a snapshot into a stable locator for your flows.

## Use it from Rust

```rust
use deskwright::{Deskwright, DeskwrightConfiguration};
use serde_json::json;

let deskwright = Deskwright::launch(&DeskwrightConfiguration::default())?;
let output = deskwright
    .call("desk_click", json!({ "app": "Notes", "role": "button", "title": "New Note" }))
    .await;
assert!(!output.is_error);
```

## Tools

| Tool | What it does |
|---|---|
| `desk_list_apps` | List running apps |
| `desk_launch_app` | Launch an app or bring it to the front (optionally open an http(s) address) |
| `desk_quit_app` | Quit an app and wait until it exits (`force` discards unsaved work) |
| `desk_window` | List, minimize, restore, close, raise, move or resize an app's windows |
| `desk_snapshot` | An app's element tree as a compact outline with refs (`e1`, `e2`, …) |
| `desk_find` | Find elements by role, title, value, placeholder or identifier |
| `desk_read` | One element's live state |
| `desk_wait_for` | Wait until an element appears, or with `state: "hidden"`, goes away |
| `desk_expect` | Assert a state (visible, enabled, checked, text, count…), retrying until it holds |
| `desk_locator` | Turn a snapshot ref into a stable locator ("pick locator") |
| `desk_click` | Click an element; `button: "right"` and `clickCount: 2` for right- and double-clicks |
| `desk_hover` | Move the pointer over an element |
| `desk_drag` | Drag one element onto another (`from`, `to`) |
| `desk_type` | Enter text into an element, or wherever keyboard focus is |
| `desk_press_keys` | Press a key or shortcut (Enter, Escape, Cmd+S…) |
| `desk_check` | Check or uncheck a checkbox, radio button or switch, verified |
| `desk_select` | Choose an option from a pop-up button |
| `desk_scroll_into_view` | Scroll an element into view |
| `desk_menu` | Choose a menu-bar item, e.g. `File > Save As…` |
| `desk_screenshot` | PNG of an element or the main display |
| `desk_read_clipboard` / `desk_write_clipboard` | Read or write the clipboard's text |
| `desk_batch` | Run up to 50 tool calls in one request |
| `desk_script` | Run AppleScript/JXA. **Off unless `--enable-script` is given** |

Every element tool accepts a `ref` from a snapshot, or a locator: `role`, `title`, `value`,
`placeholder`, `identifier`, `exact`, `within`, `app`, `nth` or `last`, and `timeoutSeconds`.

## Configuration

| Flag / environment variable | Default | Meaning |
|---|---|---|
| `--enable-script` / `DESKWRIGHT_ENABLE_SCRIPT` | off | Register `desk_script`, which can run any command as you |
| `--script-timeout-seconds` / `DESKWRIGHT_SCRIPT_TIMEOUT_SECONDS` | 30 | Script time limit (1–600) |
| `--accessibility-timeout-seconds` / `DESKWRIGHT_ACCESSIBILITY_TIMEOUT_SECONDS` | 3 | How long to wait for an unresponsive app (1–60) |
| `RUST_LOG` | `info` | Log verbosity; logs go to stderr |

## Safety

deskwright acts as you: it can click, type and send anything you could. It refuses pointer
input on covered windows and key presses into an app that isn't frontmost, keeps the script
tool off by default, and never echoes typed text (it may be a password). Review what an AI
assistant plans to do before letting it act on important apps. See [SECURITY.md](SECURITY.md).

## Platforms

| Platform | Status | Backend |
|---|---|---|
| macOS 13+ | Supported | Accessibility API (`AXUIElement`), `NSWorkspace`, AppleScript/JXA |
| Windows | Planned | UI Automation |
| Linux | Planned | AT-SPI |

Everything above the adapters (locators, auto-waiting, assertions, tools, flows, traces) is
shared, so a new platform is one crate implementing the same small interfaces as
`crates/deskwright-macos`. Browsers are already driveable as desktop apps (windows, tabs,
menus); for web page content, [Playwright](https://playwright.dev) remains the right tool.
Contributions of new backends are very welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).

## How it works

A Rust workspace in a ports-and-adapters layout: a core with no OS code (models, port traits,
services), platform adapters (macOS today, over the Accessibility API), portable input and screen-capture
adapters, an MCP layer with one type per tool, a flow runner, and a thin binary. See
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and one document per feature in
[docs/features/](docs/features/README.md).

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md). In short: run
`./scripts/check.sh` before and after your change; it runs formatting, lint, every test and
the coverage gates.

## Acknowledgments

Inspired by [Playwright](https://playwright.dev) and by
[desktop-pilot-mcp](https://github.com/VersoXBT/desktop-pilot-mcp), whose ideas this project
reimplements from scratch in Rust.

## License

[MIT](LICENSE)
