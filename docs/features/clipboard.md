# Clipboard

> Reads and writes the clipboard's text, for copy-out checks and fast paste-in.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Two common needs: checking what an app copied (Cmd+C, "Copy link"), and entering long text
quickly (write the clipboard, then Cmd+V), which is much faster than typing it key by key.

## How it works

- `desk_read_clipboard` returns the general pasteboard's string, or says it holds no text.
- `desk_write_clipboard { text }` clears the pasteboard and writes the string. The
  confirmation never echoes the text, which may be a password.
- Both tools call the `Clipboard` port directly; there is no service, since one port call is
  the whole use case (the same as `desk_list_apps`).

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| Two tools instead of one with a mode | The reader can be marked read-only, so clients can allow it without a prompt | `desk_clipboard { text? }` |
| Text only | Covers copy and paste of text, which is what flows need | Images and files |
| `NSPasteboard` through `objc2` | The same pasteboard Cmd+C and Cmd+V use, with no subprocess | `pbcopy` / `pbpaste` |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/ports/clipboard.rs` | The port |
| Adapter | `crates/deskwright-macos/src/pasteboard_clipboard.rs` | `NSPasteboard` |
| Interface | `crates/deskwright-mcp/src/tools/read_clipboard_tool.rs`, `write_clipboard_tool.rs` | The tools |

## Inputs and outputs

`desk_write_clipboard { text: "…" }` → `Put the text on the clipboard.`
`desk_read_clipboard {}` → the text itself, or `The clipboard holds no text.`

## How to extend

Rich content (images, file URLs) would be new `Clipboard` methods and output blocks.

## Testing

`window_and_app_tools_tests.rs` tests writing, reading and the empty clipboard on the fake
desktop. The adapter overwrites the user's real clipboard, so it is excluded from the
coverage gate.

## Limitations and future work

Writing replaces everything on the clipboard; the previous contents are not restored.
