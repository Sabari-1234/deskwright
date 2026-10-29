# Script Execution (Opt-in)

> Runs AppleScript or JavaScript for Automation (JXA) and returns its output. Disabled
> unless the server is started with `--enable-script`.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Scriptable apps (Finder, Mail, Safari, Music, Keynote…) can do things through their
scripting dictionaries that are awkward through the UI. AppleScript can also run shell
commands (`do shell script`), which makes this tool as powerful as a terminal.

## How it works

1. The composition root registers `ScriptTool` only when `--enable-script` /
   `DESKWRIGHT_ENABLE_SCRIPT=true` is set. When it's off, clients can't see or call the tool.
2. `ScriptTool` validates the input into a `Script` (non-empty, at most 100,000 characters)
   with the configured timeout (`--script-timeout-seconds`, default 30, maximum 600).
3. `OsaScriptRunner` starts `/usr/bin/osascript -l AppleScript|JavaScript`, writes the source to
   its stdin, drains stdout and stderr on background threads, and kills the process when the
   timeout passes.
4. Output is returned as text. A non-zero exit becomes an error result with the script's
   error text.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| Off by default, enabled in the composition root | A prompt-injected client could otherwise run any command as the user | Always on, as in the original |
| Source sent on stdin, not with `-e` | No argument-length limit, and the source doesn't appear in `ps` output | `osascript -e <source>` |
| Tool annotated destructive | Lets MCP clients ask the user before each call | No annotations |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/script.rs`, `ports/script_runner.rs` | Validated script, port |
| Adapter | `crates/deskwright-macos/src/osascript_script_runner.rs` | Process, stdin, timeout |
| Interface | `crates/deskwright-mcp/src/tools/script_tool.rs` | `desk_script` |
| Composition | `crates/deskwright/src/composition.rs`, `configuration.rs` | Opt-in registration, timeout |

## Inputs and outputs

Arguments: `code`, `language?` (`applescript` default, or `javascript`). Result: the script's
output, or `Script finished with no output.` Errors: empty or too-long source, script error
text, `script timed out after N seconds`.

## How to extend

A Windows backend could implement `ScriptRunner` with PowerShell behind the same tool.

## Testing

`script.rs` tests source validation. `osascript_script_runner.rs` runs the real `osascript`
for AppleScript and JXA output, error text, and the timeout kill.
`crates/deskwright-mcp/tests/script_tool_tests.rs` covers output, empty output, failures, and
rejected input through the fake runner. `composition.rs` tests that the tool is registered
only when enabled, and the stdio smoke test checks that the real binary hides it by default
and runs a script when `--enable-script` is set.

## Limitations and future work

Scripts run with the server's full user permissions; there is no sandbox. Keep the tool off
unless you need it, and review each script your client wants to run.
