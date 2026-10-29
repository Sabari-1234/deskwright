# Element Interaction (Click and Type)

> Clicks elements and enters text, trying the most precise method first and falling back
> to synthetic mouse and keyboard input when an app doesn't cooperate.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Apps differ in how well they support accessibility actions. Native controls handle
"press" and "set value" reliably; custom-drawn controls and many Electron apps don't. Each
action therefore has an ordered list of strategies, and the result says which one worked.

## How it works

1. `ClickTool` / `TypeTextTool` parse the `ref` and call `ClickService::click` or
   `TextEntryService::enter_text` on the blocking pool.
2. The service resolves the ref in the `ElementRegistry` and runs its `StrategyChain`.
   The chain tries each strategy in order and returns at the first success. When every
   strategy fails, the error lists each attempt and why it failed.
3. Click strategies, in the order set in the composition root:
   - `AccessibilityPressClick` runs the element's `AXPress` action. It works even when the
     window is covered.
   - `PointerClick` sends a synthetic left click at the center of the element's on-screen
     bounds. It first asks the `HitTester` port which app is topmost at that point, and
     refuses if another app's window covers the element, so the click can never land in the
     wrong app. UI drawn by a system XPC service on the app's behalf (such as a sandboxed
     app's Save panel) counts as the app's own. Before clicking, the adapter waits until the
     cursor has actually reached the target, because `enigo` clicks wherever it reads the
     cursor to be and macOS applies mouse moves asynchronously.
4. Text-entry strategies:
   - `AccessibilityValueEntry` only handles editable text roles (text field, text area, combo
     box, search field). It focuses the element, sets its value, then reads it back. If the app
     accepted the write but didn't apply it, the read-back turns that into a failure so the next
     strategy runs. An editable field whose value can't be read back (a password field) is
     trusted.
   - `KeyboardTextEntry` asks for focus, then presses the element if that didn't work, waiting
     up to 500 ms after each attempt (apps apply focus asynchronously), and types with
     synthetic key events only once the element reports keyboard focus.
5. `desk_type` without a `ref` skips both strategies and types into whatever has focus in the
   frontmost app through `KeyboardService`, refusing if the named `app` isn't frontmost. This
   covers apps whose input box isn't in the accessibility tree, such as WhatsApp's message box.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| Value entry limited to editable roles | WhatsApp's search box is static text that accepted a value write, reported success and showed nothing (found in live testing) | Trusting any write whose value can't be read back |
| Verify focus before keyboard typing | Keys go wherever focus is, so typing after a failed focus lands in the wrong place | Typing after a best-effort focus |
| Hit-test before every pointer click | Live testing showed a covered element's click landing in the window on top | Raising the window automatically (changes window order without asking) |
| System XPC UI services count as the target app | Sandboxed apps' Save and Open panels are drawn by `openAndSavePanelService`, which the hit test otherwise reported as a covering app | Allowing any non-Dock process (would let menu-bar apps' floating windows absorb clicks) |
| Wait for the cursor to arrive before clicking | Live testing showed clicks landing at the previous cursor position | A fixed sleep after every move |
| **Strategy** per interaction method | Each method is a small type with its own dependencies, testable in isolation | One function with `if` branches per method |
| **Chain of Responsibility** via the generic `StrategyChain` | One fallback implementation shared by click and type, with complete error reporting | Per-app routing tables (the original's unused "smart router") |
| Strategy order set in the composition root | Changing priority, or adding a strategy, touches one place and no service code | Order hard-coded in the services |
| Read-back verification for value entry | Catches apps that silently ignore value writes | Trusting the write's success code |
| The type tool never echoes the text | The text may be a password | Echoing for confirmation |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Application | `crates/deskwright-core/src/services/interaction/strategy_chain.rs` | `StrategyChain`, `NamedStrategy`, `InteractionOutcome` |
| Application | `crates/deskwright-core/src/services/interaction/click_strategies.rs`, `click_service.rs` | Click strategies and use case |
| Application | `crates/deskwright-core/src/services/interaction/text_entry_strategies.rs`, `text_entry_service.rs` | Text-entry strategies and use case |
| Domain | `crates/deskwright-core/src/ports/element_actions.rs`, `ports/input_synthesizer.rs` | Ports used by the strategies |
| Adapter | `crates/deskwright-macos/src/accessibility/ax_element_actions.rs` | Press, focus, set value |
| Adapter | `crates/deskwright-portable/src/enigo_input_synthesizer.rs` | Synthetic typing and clicks |
| Interface | `crates/deskwright-mcp/src/tools/click_tool.rs`, `tools/type_text_tool.rs` | `desk_click`, `desk_type` |

## Inputs and outputs

`desk_click { ref }` returns `Clicked e5 using accessibility press.`
`desk_type { ref, text }` returns `Entered text into e5 using keyboard typing.`
Failures list every strategy tried, for example
`could not click the element; every strategy failed: accessibility press: …; pointer click: …`.

Double-clicks and right-clicks have their own strategy chains; see
[pointer gestures](pointer-gestures.md).

## How to extend

Add a strategy, such as "paste from clipboard", by implementing
`ClickStrategy` or `TextEntryStrategy` (plus `NamedStrategy`) in the matching
`*_strategies.rs` file and inserting it into the list in
`crates/deskwright/src/composition.rs`. No service code changes.

## Testing

`strategy_chain.rs` tests first-success and all-failed reporting. `click_service.rs` tests
press, pointer fallback at the computed center, and total failure. `text_entry_service.rs`
tests direct value entry and the fallback to typing when the app ignores the value.
`enigo_input_synthesizer.rs` tests permission-error mapping and coordinate rounding.
`crates/deskwright-mcp/tests/element_tools_tests.rs` checks the tool messages, that typed text is
never echoed, and the rejection of malformed and unknown refs. Real presses and synthetic input
change app state, so they are excluded from the coverage gate (see `docs/TESTING.md`).

## Limitations and future work

- Keyboard typing inserts at the cursor instead of replacing existing text.
- A covered element that has no press action can't be clicked until its window is brought to
  the front. There is no tool yet to bring an app to the front, or to press keys such as
  Enter or Escape.
- `enigo` needs the Accessibility permission. Its connection opens on first use, so granting
  the permission while the server runs takes effect on the next call.
