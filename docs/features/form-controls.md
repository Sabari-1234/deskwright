# Form Controls (Check, Select, Scroll Into View)

> Puts checkboxes and switches into a given state, chooses options from pop-up buttons, and
> scrolls elements into view, like Playwright's `check()`, `selectOption()` and
> `scrollIntoViewIfNeeded()`.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

"Click the checkbox" is only safe when you know its current state, and "click the pop-up, then
click the item" takes two calls and a snapshot in between. These tools state the intent
directly and verify it.

## How it works

1. **Check** (`desk_check { target, checked? }`): `ToggleService::set_checked` reads the
   element's value (`1`/`true` means checked). If it already matches, nothing is clicked.
   Otherwise it clicks through `ClickService` (so the usual press-then-pointer fallback
   applies) and polls the value for up to 500 ms, failing if the state never changed.
2. **Select** (`desk_select { target, option }`): `OptionService::select` presses the pop-up,
   polls up to 2 s for an item with that title (ignoring case) using `MenuNavigator`, and
   presses it. AppKit exposes the open menu under the pop-up the first time and under the app
   itself after that (found in live testing in Finder), so both places are searched. When
   there is no such item, the error lists the available titles and the open menu is closed
   with `AXCancel`; left open, it kept the pop-up from ever opening again.
3. **Scroll into view** (`desk_scroll_into_view { target }`): `ScrollService` performs the
   element's `AXScrollToVisible` action. Most controls outside scroll areas don't support it;
   for those, an element already on screen counts as success ("if needed"), and only an
   off-screen one fails.
4. All three accept a `ref` or a locator, which auto-waits like the other element tools.

`MenuNavigator` is shared with [menu navigation](menu-navigation.md): it finds items directly
under a parent or one level down inside an untitled menu container, which is how both
menu-bar menus and pop-up button menus are built.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| Check reads the state first | Repeating a step is safe, and a flow can say "make sure it's on" | A plain click (toggles blindly) |
| Verify the state after clicking | Catches clicks the app ignored, like the value read-back in text entry | Trusting the click |
| Select lists available options on failure | The client can correct the title without another snapshot | A bare "not found" |
| Search the app's open menus too, and cancel on failure | Live testing showed the menu moving to the app level and a left-open menu blocking later presses | Pressing the pop-up again to close it (left AppKit stuck) |
| Unscrollable but visible counts as scrolled | Matches Playwright's "if needed"; checkboxes in panels reject the action | Failing |
| `MenuNavigator` extracted from `MenuService` | Pop-up menus and menu-bar menus have the same shape; one search, one set of tests | Duplicating the search |
| Timeouts injected from the composition root | Tests run with zero wait, production with real settle times | Constants inside the services |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Application | `crates/deskwright-core/src/services/interaction/toggle_service.rs` | Check and uncheck with verification |
| Application | `crates/deskwright-core/src/services/option_service.rs` | Choose a pop-up option |
| Application | `crates/deskwright-core/src/services/menu_navigator.rs` | Find menu items by title |
| Application | `crates/deskwright-core/src/services/interaction/scroll_service.rs` | Scroll into view |
| Adapter | `crates/deskwright-macos/src/accessibility/ax_element_actions.rs` | `AXScrollToVisible` |
| Interface | `crates/deskwright-mcp/src/tools/check_tool.rs`, `select_tool.rs`, `scroll_into_view_tool.rs` | The tools |

## Inputs and outputs

- `desk_check { role: "checkbox", title: "Wi-Fi" }` → `e7 is now checked (using accessibility press).`
  or `e7 was already checked.`; `checked: false` unchecks.
- `desk_select { role: "popupbutton", title: "Format", option: "PNG" }` →
  `Selected "PNG" in e3.`, or the error `… no option "GIF"; available: JPEG, PNG`.
- `desk_scroll_into_view { ref }` → `Scrolled e12 into view.`

## How to extend

Support another kind of option list (for example a combo box's list) by teaching
`OptionService` a second way to open it, as a strategy if the ways multiply.

## Testing

`toggle_service.rs` tests checking, the no-op when already in state, and a click that didn't
change the state. `option_service.rs` tests opening and pressing, the available-options error,
and an element without options. `menu_navigator.rs` tests direct and nested lookup and title
listing. `crates/deskwright-mcp/tests/pointer_and_control_tools_tests.rs` tests the tools end to end
on the fake desktop.

## Limitations and future work

- Only pop-up buttons and menu buttons are supported, not combo-box lists or table rows.
- Mixed-state checkboxes (value `2`) count as unchecked.
