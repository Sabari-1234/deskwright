# Changelog

## 0.1.0 (2026-10-29)

First public release.

- MCP server with 23 tools (plus opt-in AppleScript/JXA) for driving desktop apps through the accessibility API (macOS): snapshots,
  search, reading, clicking (right- and double-click), hovering, dragging, typing, keys,
  menus, checkboxes, pop-up options, scrolling, windows, app launching and quitting, the
  clipboard, screenshots, batches, and opt-in AppleScript/JXA.
- Playwright-style locators (role, title, value, placeholder, identifier, exact, within,
  nth/last) with auto-waiting, and `desk_expect` auto-retrying assertions.
- `deskwright` CLI: `serve`, `tools`, `call` and `run` for JSON flows, with text, JSON and
  HTML-trace reports.
- Rust library facade `deskwright::Deskwright`.
- Install with `npx -y deskwright`, `brew install sabari-1234/tap/deskwright`, the shell
  installer, or the prebuilt Apple Silicon and Intel binaries on GitHub Releases.
