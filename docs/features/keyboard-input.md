# Keyboard Input

> Presses a key or keyboard shortcut, such as Enter, Escape or Cmd+S, in the frontmost app.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Many actions have no clickable element: submitting a search, closing a menu, shortcuts like
Cmd+L or Cmd+Shift+G, and keyboard-driven apps such as Blender (F3 search, Shift+A).

## How it works

1. `PressKeysTool` parses `keys` into a `KeyChord`: modifiers then one key, joined with `+`,
   ignoring case (`cmd`/`command`/`⌘`, `ctrl`, `opt`/`alt`, `shift`; named keys such as
   `Enter`, `Escape`, `Tab`, arrows, `PageUp`; or any single character).
2. `KeyboardService::press` resolves the target app and refuses unless it is frontmost, because
   keys always go to the frontmost app. Without `app`, the frontmost app is the target.
   `KeyboardService::type_text`, used by `desk_type` without a ref, applies the same rule to
   typed text.
3. `EnigoInputSynthesizer::press_chord` holds the modifiers, taps the key, and always releases
   the modifiers, even if the tap failed, so none stays held down.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| `KeyChord` value type | Parsing and validation happen once at the boundary; adapters get typed keys | Passing strings to the adapter |
| Refuse when the target app isn't frontmost | Same safety rule as pointer clicks: input must not land in the wrong app | Sending regardless |
| `press_chord` added to `InputSynthesizer` | Same capability (synthetic input) and same adapter as typing and clicks | A separate keyboard port |
| Marked destructive | Shortcuts act immediately (Cmd+Q quits) | Read-write without the hint |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/key_chord.rs` | Chord parsing and display |
| Domain | `crates/deskwright-core/src/ports/input_synthesizer.rs` | `press_chord` |
| Application | `crates/deskwright-core/src/services/keyboard_service.rs` | Frontmost check, chords and typed text |
| Adapter | `crates/deskwright-portable/src/enigo_input_synthesizer.rs` | Key mapping and events |
| Interface | `crates/deskwright-mcp/src/tools/press_keys_tool.rs` | `desk_press_keys` |

## Inputs and outputs

Arguments: `keys`, `app?`. Result: `Pressed Cmd+Shift+G in TextEdit.` Errors: unknown modifier
or key, app not running, app not frontmost.

## How to extend

Add a named key to `NamedKey` (with its aliases and label) and map it in `chord_key` in the
enigo adapter.

## Testing

`key_chord.rs` (aliases, ordering, display, invalid chords), `keyboard_service.rs` (frontmost,
background refusal, no app named), the enigo mapping tests, and
`crates/deskwright-mcp/tests/launch_and_keys_tools_tests.rs`. Verified live: Cmd+T and Enter in
Chrome.

## Limitations and future work

- One chord per call; use `desk_batch` for sequences.
- To type free text without a ref, use `desk_type` with only `text` (and `app`).
