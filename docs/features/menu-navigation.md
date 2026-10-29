# Menu Navigation

> Chooses a menu-bar item by path, such as `File > Save As…`, without opening menus on screen.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Menu commands are the most reliable way to trigger many app features, and on macOS the
accessibility tree exposes the whole menu bar even when the menus are closed.

## How it works

1. `MenuTool` parses `path` into a `MenuPath` (items separated by `>`, trimmed, none empty).
2. `MenuService::choose` checks the permission, resolves the app, and gets its menu bar from
   `MenuBarLocator`.
3. For each title in the path, it searches the current element's children, ignoring case. An
   untitled child is a menu container (menu bar item → menu → items), so its children are
   searched too.
4. It presses the final item through `ElementActions::press`. When a title is missing, the error
   names that title and the full path.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| Navigation logic in the core, over generic ports | The walk is testable with fakes and reusable on other OSes | Implementing menus inside the macOS adapter |
| `MenuBarLocator` as its own small port | Keeps `ElementTreeReader` focused on generic trees (Interface Segregation) | Adding `menu_bar` to the tree reader |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/menu_path.rs` | Path parsing and display |
| Domain | `crates/deskwright-core/src/ports/menu_bar_locator.rs` | Menu bar port |
| Application | `crates/deskwright-core/src/services/menu_service.rs` | Path walk and press |
| Application | `crates/deskwright-core/src/services/menu_navigator.rs` | Finding an item by title, shared with [form controls](form-controls.md) |
| Adapter | `crates/deskwright-macos/src/accessibility/ax_menu_bar_locator.rs` | `AXMenuBar` lookup |
| Interface | `crates/deskwright-mcp/src/tools/menu_tool.rs` | `desk_menu` |

## Inputs and outputs

Arguments: `path`, `app?`. Result: `Chose menu item 'File > Save As…'.` Errors: invalid path,
missing item (`menu item 'Print' was not found while following 'File > Print'`), missing
permission, app not found.

## How to extend

Support item shortcuts or disabled-item checks by reading more attributes in `find_item`.

## Testing

`menu_path.rs` tests parsing. `menu_service.rs` tests walking through untitled menu
containers and the missing-item error. `crates/deskwright-mcp/tests/app_tools_tests.rs` tests the
tool's messages and errors. `crates/deskwright-macos/tests/accessibility_integration_test.rs` reads
Finder's real menu bar when Accessibility permission is granted.

## Limitations and future work

- A background app may not populate all of its menus; for example, TextEdit's
  **Format > Make Plain Text** is only found while TextEdit is the frontmost app.
- Titles must match exactly, apart from case: `Save As...` with three dots doesn't match
`Save As…` with an ellipsis character.
