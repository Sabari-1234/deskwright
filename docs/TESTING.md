# Testing

## The one command

```bash
./scripts/check.sh
```

It runs, in order, and stops at the first failure:

1. `cargo fmt --all --check`, for formatting
2. `cargo clippy --workspace --all-targets -- -D warnings`, for lint (warnings are errors)
3. every unit and integration test, under coverage instrumentation (`cargo llvm-cov`)
4. coverage gates: **≥ 95% lines** for the whole project and **≥ 98% lines** for `deskwright-core`

One-time setup: `brew install cargo-llvm-cov && rustup component add llvm-tools-preview`.

## Rules for every change, for people and AI agents

1. Run `./scripts/check.sh` **before** changing anything. If it already fails, report that first
   so pre-existing failures aren't blamed on the new work.
2. Write tests with the code: the main path and every failure path of each new behavior. For a
   bug fix, start with a test that fails because of the bug.
3. Run `./scripts/check.sh` **again before calling the work done**. It must pass.
4. Never make it pass by weakening it. Don't delete, `#[ignore]`, or loosen existing tests,
   and don't lower the coverage gates in `scripts/check.sh` (they only go up). If an existing
   test fails because the intended behavior changed, update that test on purpose and say so.

## Test layout

| Where | What | Runs against |
|---|---|---|
| `#[cfg(test)] mod tests` in each `deskwright-core` file | Domain rules, the registry, every service and strategy | `FakeDesktop` |
| `crates/deskwright-mcp/src/*` unit tests | Protocol mapping, server metadata, blocking-pool helper | — |
| `crates/deskwright-mcp/tests/*_tests.rs` | Every tool through the `ToolRegistry`: arguments, results, errors, batch rules, launching and key presses, pointer gestures and form controls, windows, quitting, the clipboard, expectations | `FakeDesktop` via `tests/support/mod.rs` |
| `crates/deskwright-runner/src/*` unit tests and `tests/flow_runner_tests.rs` | Flow parsing and limits, the pass rule, every reporter's exact output, running, timing, stopping, trace screenshots | A scripted `ToolDispatcher` and a ticking `Clock` |
| `crates/deskwright-macos/src/*` unit tests | AX error mapping, Core Foundation value conversion, attribute parsing, element handles | Real Core Foundation (no permission needed) |
| `crates/deskwright-macos/src/osascript_script_runner.rs` tests | Script output, failures, timeout | Real `/usr/bin/osascript` (no permission needed) |
| `crates/deskwright-macos/src/workspace_app_directory.rs` tests | Running-app listing | Real `NSWorkspace` (no permission needed) |
| `crates/deskwright-macos/tests/accessibility_integration_test.rs` | Tree reading, menu bar and hit testing on the real Finder, read-only | Real Accessibility API; **skipped** without permission |
| `crates/deskwright-portable/src/*` unit tests | Display clipping, PNG encoding, input error mapping, coordinate rounding | — |
| `crates/deskwright/src/*` unit tests | Flag and CLI parsing, tool registration (script opt-in), the `Deskwright` facade, command output, macOS adapter factory | `FakeDesktop`, real adapters |
| `crates/deskwright/tests/stdio_protocol_smoke_test.rs` | The real binary over stdio: handshake, tool list, tool calls, unknown tool, script opt-in, clean shutdown | Real process |
| `crates/deskwright/tests/cli_tests.rs` | The real binary's `tools`, `call` and `run` commands: exit codes, text and JSON reports, HTML traces, invalid flows | Real process (no permission needed) |

## The shared fake

`deskwright_core::testing::FakeDesktop` implements every port in memory: apps, an element tree, a
menu bar, windows, a clipboard, and configurable failures (`make_press_fail`,
`make_set_value_ignored`, `make_unreadable`, `make_show_menu_fail`, `make_covered`,
`make_ignore_quit`, `deny_accessibility_permission`, `set_script_result`). Checkboxes toggle
when pressed, and `register_all` registers several elements in one snapshot. It records every
side effect as a `RecordedAction`. Other crates enable it with the `test-support` feature in
their `[dev-dependencies]`. Extend the fake rather than writing ad-hoc mocks.

## Coverage exclusions

Excluded from the gates because every line is a call into an API that needs a real macOS
permission. Their logic has been moved into tested modules:

| File | Why excluded | Logic tested in |
|---|---|---|
| `deskwright-macos/src/accessibility/ax_element_actions.rs` | Press/focus/set value need Accessibility permission and change app state | `ax_error_code.rs` |
| `deskwright-macos/src/accessibility/ax_menu_bar_locator.rs` | Needs Accessibility permission | integration test (when permitted) |
| `deskwright-macos/src/accessibility/ax_hit_tester.rs` | Needs Accessibility permission | `ax_element.rs` tests, integration test (when permitted) |
| `deskwright-macos/src/accessibility/ax_permission.rs` | One call to `AXIsProcessTrusted` | — |
| `deskwright-macos/src/accessibility/ax_window_controller.rs` | Minimizes, moves, resizes and closes the user's real windows | `ax_value_conversion.rs` (point and size round trip), `window_service.rs` |
| `deskwright-macos/src/running_app_terminator.rs` | Quits real apps | `app_quit_service.rs` |
| `deskwright-macos/src/pasteboard_clipboard.rs` | Overwrites the user's clipboard | the clipboard tool tests on the fake desktop |
| `deskwright-portable/src/enigo_input_synthesizer.rs` | Sends real input events | its own unit tests for error mapping and rounding |
| `deskwright-portable/src/xcap_screen_capturer.rs` | Needs Screen Recording permission | `display_region.rs`, `png_encoding.rs` |
| `deskwright-core/src/testing/fake_desktop.rs` | Test support, not product code | — |

## Running the permission-gated tests

Grant **Accessibility** to the app running the tests (Terminal, iTerm, VS Code…), then run:

```bash
cargo test -p deskwright-macos --test accessibility_integration_test -- --nocapture
```

Without the permission, they print `skipped: …` and pass.
