# deskwright

Notes for AI coding agents (and people) working in this repository. deskwright is Playwright
for desktop apps: an MCP server, a CLI flow runner and a Rust library that drive native apps
through the OS accessibility API, synthetic input, and screenshots. macOS is supported today
(through its Accessibility API and AppleScript); new platforms are new adapter crates.

## Standards

Follow [CONTRIBUTING.md](CONTRIBUTING.md). Every feature gets `docs/features/<feature>.md`;
keep `docs/features/README.md`, `docs/ARCHITECTURE.md` and `docs/TESTING.md` current.

## Regression check (required before and after every change)

```bash
./scripts/check.sh
```

Formatting, clippy with warnings as errors, every test, and the coverage gates (≥ 95% lines
project-wide, ≥ 98% for `deskwright-core`). Run it before starting, to get a baseline, and again
before reporting done; it must pass. Every new behavior needs unit tests for its main and
failure paths, using `deskwright_core::testing::FakeDesktop`. Never delete, `#[ignore]`, or weaken
existing tests, and never lower the gates. See `docs/TESTING.md`.

## Architecture (ports and adapters)

| Crate | Layer | May depend on |
|---|---|---|
| `crates/deskwright-core` | Domain + application: models, port traits, element registry, services | std, `thiserror`, `serde` only. No OS, MCP or async-runtime code. |
| `crates/deskwright-macos` | Infrastructure: macOS-only adapters (Accessibility API, NSWorkspace, osascript) | `deskwright-core` |
| `crates/deskwright-portable` | Infrastructure: cross-platform adapters (`enigo` input, `xcap` screen capture) | `deskwright-core` |
| `crates/deskwright-mcp` | Interface: MCP tools (one `TypedTool` type per file), tool registry, `rmcp` server adapter | `deskwright-core` |
| `crates/deskwright-runner` | Interface: flow files, the flow runner, reporters (text, JSON, HTML trace) | `deskwright-mcp` |
| `crates/deskwright` | Composition root: the `Deskwright` library facade, config, platform adapter factory, wiring; the CLI binary (`serve`, `tools`, `call`, `run`) | all crates |

- Core ports are small, focused traits. OS-specific adapters live only in `deskwright-macos`, so a
  future `deskwright-windows` crate can implement the same ports and reuse `deskwright-portable`.
- Only `crates/deskwright/src/platform.rs` names concrete adapters, and only
  `crates/deskwright/src/composition.rs` constructs services and tools.
- `unsafe` is confined to `deskwright-macos`'s system-call modules (`src/accessibility/`,
  `src/process_list.rs`, and the framework constants read in `src/pasteboard_clipboard.rs`), and every block needs a `// SAFETY:` comment.
- Dispatching tools by name uses a Registry; fallbacks (for example, setting a value via
  Accessibility, then synthetic typing) use Strategy + Chain of Responsibility. Never use
  growing `match` ladders outside the composition root.
- The AppleScript/JXA tool is off by default and only registered with an explicit opt-in flag.
- stdout carries the MCP protocol; all logging goes to stderr via `tracing`.

## Commands

If Rust came from Homebrew's keg-only `rustup`, it isn't on `PATH` by default.
`scripts/check.sh` adds it automatically; for ad-hoc `cargo` commands:

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
```

Tests need no macOS permissions. The read-only Accessibility integration tests skip
themselves unless the test process has Accessibility permission.
