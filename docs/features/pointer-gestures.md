# Pointer Gestures (Double-click, Right-click, Hover, Drag)

> Mouse gestures beyond a single click: double-click, right-click, hover and drag-and-drop,
> each aimed at an element's on-screen center and refused when another app covers it.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Playwright's `dblclick()`, `click({ button: 'right' })`, `hover()` and `dragTo()` have no
direct accessibility equivalent for most of these gestures, so they need synthetic mouse
input aimed at the right point, with the same safety rule as pointer clicks.

## How it works

1. `PointerTarget` is the one place that turns an element into a point: the center of its
   on-screen bounds (`ElementTreeReader::on_screen_bounds`), refused unless the `HitTester`
   reports the element's own app on top there (`safe_point`). `visible_center` skips the
   cover check, for drop targets that may belong to another app.
2. `desk_click` takes `button: "left" | "right"` and `clickCount: 1 | 2` and maps them to a
   `ClickKind`. `ClickService` holds a separate `StrategyChain` for each kind
   (`ClickStrategies`):
   - primary: `AccessibilityPressClick`, then `PointerClick(Single)`;
   - double: `PointerClick(Double)` only, because there is no accessibility double-click;
   - secondary: `AccessibilityMenuClick` (`AXShowMenu`, which needs no pointer), then
     `PointerClick(Secondary)`.
3. `PointerGestureService::hover` moves the pointer to the safe point (`desk_hover`).
4. `PointerGestureService::drag` resolves both targets (`desk_drag { from, to }`), presses at
   the source's safe point, waits 60 ms, moves to the destination's center in 12 steps
   (`drag_path`), and always releases the button, even when a move fails. The pause and steps
   matter: `enigo` only sends "mouse dragged" events once macOS reports the button as held,
   and live testing showed a single jump being treated as a click on the source.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| One `PointerTarget` for clicks, hover and drag | The bounds and cover check exist once | Copying the hit-test logic into each gesture |
| `PointerClick` parameterized by `PointerGesture` | Three gestures share one strategy type and its tests | Three near-identical strategy types |
| A strategy chain per click kind | Each kind gets its own fallbacks without `if`s in the service | Passing the kind into every strategy |
| Drag resolves the source element before locating the destination | Locating takes a new snapshot, which invalidates older refs | Requiring refs for both ends |
| Drop target isn't cover-checked | Dropping into another app (Finder → Mail) is a normal use | Refusing cross-app drags |
| Drag always releases the button | A stuck button breaks every later mouse action | Releasing only on success |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Application | `crates/deskwright-core/src/services/interaction/pointer_target.rs` | Safe point and visible center |
| Application | `crates/deskwright-core/src/services/interaction/click_strategies.rs` | `AccessibilityMenuClick`, `PointerClick` gestures |
| Application | `crates/deskwright-core/src/services/interaction/click_service.rs` | `ClickKind`, `ClickStrategies` |
| Application | `crates/deskwright-core/src/services/interaction/pointer_gesture_service.rs` | Hover and drag |
| Domain | `crates/deskwright-core/src/ports/input_synthesizer.rs`, `ports/element_actions.rs` | `double_click_at`, `right_click_at`, `move_pointer_to`, `drag`, `show_menu` |
| Adapter | `crates/deskwright-portable/src/enigo_input_synthesizer.rs` | The mouse gestures |
| Adapter | `crates/deskwright-portable/src/drag_path.rs` | The intermediate drag points (tested) |
| Adapter | `crates/deskwright-macos/src/accessibility/ax_element_actions.rs` | `AXShowMenu` |
| Interface | `crates/deskwright-mcp/src/tools/click_tool.rs`, `hover_tool.rs`, `drag_tool.rs` | `desk_click`, `desk_hover`, `desk_drag` |

## Inputs and outputs

- `desk_click { ref, clickCount: 2 }` → `Clicked e4 using pointer double-click.`
- `desk_click { role: "cell", title: "Report.pdf", button: "right" }` →
  `Clicked e9 using accessibility menu.`
- `desk_hover { ref }` → `Hovering over e4.`
- `desk_drag { from: { title: "Report.pdf", app: "Finder" }, to: { title: "Archive", app: "Finder" } }`
  → `Dragged the element onto the drop target.`

`button: "right"` with `clickCount: 2`, or any `clickCount` other than 1 or 2, is rejected.

## How to extend

A new gesture (such as a triple-click) is a new `PointerGesture` variant and
`InputSynthesizer` method, plus a `ClickKind` and chain if it is a kind of click.

## Testing

`click_service.rs` tests the double-click point and right-click order (accessibility menu,
then pointer fallback). `pointer_gesture_service.rs` tests hover, drag, the covered-source and
invisible-destination refusals, and that a locator destination doesn't invalidate a ref source.
`crates/deskwright-mcp/tests/pointer_and_control_tools_tests.rs` tests the tool messages and
argument validation. The enigo gestures themselves move the real mouse, so they're excluded
from the coverage gate.

## Limitations and future work

- Hover only moves the pointer; reading the tooltip that appears is a separate
  `desk_wait_for`.
