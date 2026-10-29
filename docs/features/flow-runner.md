# Flow Runner, Reports and Traces

> Runs flow files (JSON lists of tool calls) like a Playwright test runner: PASS/FAIL per
> flow, timing per step, JSON results for CI, and an HTML trace with a screenshot after each
> step.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

MCP clients drive the desktop one call at a time. Repeatable automation (regression checks,
demos, benchmarks) needs the same steps saved in a file, run unattended, timed, and reported,
with enough evidence to see what went wrong.

## How it works

1. `deskwright run <flows…>` reads and validates every flow before running any
   (`Flow::parse`: known fields only, 1–500 steps), so a broken last file doesn't fail after
   the first flows have already changed the desktop.
2. `FlowRunner::run` dispatches each `FlowStep { tool, params, name? }` through a
   `ToolDispatcher` (the same tool registry the MCP server uses) and times it with the
   injected `Clock`. It stops at the first failure unless the flow sets
   `"continueOnError": true`; the report counts the skipped steps.
3. With `--trace <file>`, the runner calls `desk_screenshot` after every step and the
   `HtmlTraceReporter` writes one self-contained HTML page: each flow, each step with its
   timing and full output, failed steps opened, and the screenshots inline (a small version of
   Playwright's trace viewer). Screenshots need the Screen Recording permission; without it
   the trace has no images.
4. The console report comes from the chosen reporter: `TextReporter` (default) or
   `JsonReporter` (`--reporter json`). `--repeat N` runs every flow N times, which turns the
   text report into a benchmark of each step.
5. The exit code is 1 when any flow failed.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| Its own crate depending only on `deskwright-mcp` | The runner works over any `ToolDispatcher`, so its tests use a scripted dispatcher and no OS | Putting it in the binary |
| Steps shaped like `desk_batch` actions | A batch an AI client built can be saved as a flow unchanged | A new step language |
| Reporters as a **Strategy** (`Reporter` trait) | A new format (JUnit, Markdown) is one new type | A `match` on the format inside the runner |
| Injected `Clock` | Step timings are deterministic in tests | Calling `Instant::now()` directly |
| JSON flow files | No new dependency; same format as MCP arguments | YAML |
| Validate every flow first | Fail before touching the desktop | Failing midway |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Interface | `crates/deskwright-runner/src/flow.rs` | `Flow`, `FlowStep`, parsing and limits |
| Interface | `crates/deskwright-runner/src/flow_runner.rs` | Running, timing, trace screenshots |
| Interface | `crates/deskwright-runner/src/report.rs` | `FlowReport`, `StepReport` |
| Interface | `crates/deskwright-runner/src/clock.rs` | `Clock`, `SystemClock` |
| Interface | `crates/deskwright-runner/src/reporters/*.rs` | Text, JSON and HTML trace reporters |
| Composition | `crates/deskwright/src/commands/run_command.rs` | `run`: loading, repeating, writing the trace |

## Inputs and outputs

```
$ deskwright run send.json --repeat 2
PASS Write a note (412.3 ms)
    1. desk_launch_app                 97.4 ms  ok: …
    2. type                             15.1 ms  ok: Entered text into e338 using accessibility value.
    3. desk_expect                     12.0 ms  ok: Passed: role 'textarea' to contain the text "hello" (e341).
…
2 passed, 0 failed
```

## How to extend

Add a reporter by implementing `Reporter` in `crates/deskwright-runner/src/reporters/`, then a
`ReportFormat` value in `crates/deskwright/src/cli.rs` and a line in
`run_command::reporter_for`.

## Testing

`flow.rs` tests parsing, labels and the limits. `report.rs` tests the pass rule and the
millisecond serialization. Each reporter has exact-output tests, including HTML escaping.
`crates/deskwright-runner/tests/flow_runner_tests.rs` tests ordering, timing with a ticking clock,
stopping and continuing after failures, and trace screenshots, using a scripted dispatcher.
`crates/deskwright/tests/cli_tests.rs` runs the real binary on flows that need no
permissions.

## Limitations and future work

- Video recording is not implemented: `xcap`'s recorder yields raw frames and would need a
  video encoder dependency; per-step screenshots in the trace cover most debugging needs.
- Steps can't use values from earlier steps (such as a ref returned by a snapshot); locators
  make that rarely necessary.
- Traces embed full-resolution PNG screenshots, about 2 MB per step on a Retina display.
- No parallel runs: flows drive one shared desktop, so they run one after another.
