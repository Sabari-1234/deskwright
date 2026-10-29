# Feature Index

| Feature | Tool(s) | Summary |
|---|---|---|
| [App discovery](app-discovery.md) | `desk_list_apps` | Lists running apps and resolves which app a request targets |
| [App launching](app-launching.md) | `desk_launch_app` | Launches or activates an app and waits until it is frontmost |
| [App quitting](app-quitting.md) | `desk_quit_app` | Quits an app and waits until it has exited |
| [Window management](window-management.md) | `desk_window` | Lists, minimizes, restores, closes, raises, moves and resizes windows |
| [Clipboard](clipboard.md) | `desk_read_clipboard`, `desk_write_clipboard` | Reads and writes the clipboard's text |
| [Keyboard input](keyboard-input.md) | `desk_press_keys` | Presses keys and shortcuts in the frontmost app |
| [Locators](locators.md) | `desk_click`, `desk_type`, `desk_read` | Act on elements by role/title/value with auto-wait, like Playwright |
| [Element waiting](element-waiting.md) | `desk_wait_for` | Waits for elements to appear (or go away) instead of fixed delays |
| [Locator suggestion](locator-suggestion.md) | `desk_locator` | Turns a ref into the most stable locator, like Playwright's "pick locator" |
| [Expectations](expectations.md) | `desk_expect` | Auto-retrying assertions: visible, enabled, checked, text, count… |
| [UI snapshot](ui-snapshot.md) | `desk_snapshot` | Reads an app's element tree and hands out refs for the other tools |
| [Element search](element-search.md) | `desk_find` | Finds elements by role, title and value |
| [Element reading](element-reading.md) | `desk_read` | Reads one element's live state by ref |
| [Element interaction](element-interaction.md) | `desk_click`, `desk_type` | Clicks and enters text, with accessibility-first fallbacks |
| [Pointer gestures](pointer-gestures.md) | `desk_click` (`button`, `clickCount`), `desk_hover`, `desk_drag` | Double-click, right-click, hover and drag-and-drop |
| [Form controls](form-controls.md) | `desk_check`, `desk_select`, `desk_scroll_into_view` | Checkboxes, pop-up options and scrolling, verified |
| [Menu navigation](menu-navigation.md) | `desk_menu` | Chooses a menu-bar item by path |
| [Screenshots](screenshots.md) | `desk_screenshot` | Captures an element or the main display as PNG |
| [Script execution](script-execution.md) | `desk_script` | Runs AppleScript/JXA (opt-in) |
| [Batch execution](batch-execution.md) | `desk_batch` | Runs several tool calls in one request |
| [MCP server and tool registry](mcp-server.md) | all | How tools are defined, registered and served; how to add one |
| [Command line and library API](command-line.md) | `serve`, `tools`, `call`, `run`; `Deskwright` | The CLI commands and the Rust facade |
| [Flow runner, reports and traces](flow-runner.md) | `deskwright run` | Runs flow files with timings, JSON results and HTML traces |
