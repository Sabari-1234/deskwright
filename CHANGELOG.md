# Changelog

## Unreleased

- `desk_screenshot` captures a single window with `app` (and `title`/`index`) from the
  window's own pixels, so windows in front of it don't appear; off-screen windows are refused
  instead of capturing another app.
- Screenshots default to point resolution (a quarter of the pixels on Retina, like
  Playwright's `scale: "css"`): 842 KB → 387 KB for a full display. `resolution: "pixels"`
  keeps full detail.
- `format: "jpeg"` (with `quality`) makes screenshots 2.5–6× smaller; flow traces now use it.

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
