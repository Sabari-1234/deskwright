# Screenshots

> Captures one element's on-screen area, or the whole main display, as a PNG.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Some content isn't in the accessibility tree: images, canvases, video, games. Screenshots
cover those cases. Snapshots stay the main way to understand UI because they're faster
and structured.

## How it works

1. `ScreenshotTool` maps an optional `ref` to a `ScreenshotTarget` (`Element` or `MainDisplay`).
2. For an element, `ScreenshotService` resolves the ref and gets its on-screen bounds from
   `ElementTreeReader::on_screen_bounds`. Zero-size or off-screen elements are rejected.
3. `XcapScreenCapturer` finds the display under the region's center, clips the region to that
   display, captures it, and encodes it as PNG.
4. The tool returns the PNG as an MCP image block (base64).

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| `xcap` in the cross-platform `deskwright-portable` crate | Works on macOS, Windows and Linux, so every backend reuses it | Shelling out to `screencapture`, or the deprecated `CGWindowListCreateImage` |
| `on_screen_bounds` as a default method on the tree-reader port | Shared by the pointer click and screenshots without duplicating the "has visible bounds" rule | A free helper function |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/ports/screen_capturer.rs`, `model/png_image.rs` | Port and image type |
| Application | `crates/deskwright-core/src/services/screenshot_service.rs` | Target resolution |
| Adapter | `crates/deskwright-portable/src/xcap_screen_capturer.rs` | Thin calls into `xcap` |
| Adapter | `crates/deskwright-portable/src/display_region.rs` | Clipping a region to one display |
| Adapter | `crates/deskwright-portable/src/png_encoding.rs` | PNG encoding |
| Interface | `crates/deskwright-mcp/src/tools/screenshot_tool.rs` | `desk_screenshot` |

## Inputs and outputs

Argument: `ref?`. Result: one `image/png` content block. Errors: unknown ref, element without
bounds, capture failure (usually a missing Screen Recording permission).

## How to extend

Capturing a specific window could be another `ScreenshotTarget` variant backed by a new
`ScreenCapturer` method.

## Testing

`screenshot_service.rs` tests that the element's exact bounds are captured and that elements
without bounds are rejected. `display_region.rs` tests clipping (fully on screen, partly off,
a secondary display at negative coordinates, and off-display regions). `png_encoding.rs`
round-trips pixels through PNG. `crates/deskwright-mcp/tests/element_tools_tests.rs` checks both
targets return an image block. The `xcap` calls themselves need Screen Recording permission
and are excluded from the coverage gate (see `docs/TESTING.md`).

## Limitations and future work

- Captures what is on screen, so an element covered by another window shows the covering
  window's pixels. Capturing the element's own window would avoid this.
- An element that spans two displays is captured only on the display under its center.
- Without the Screen Recording permission, macOS may return an image with other apps'
  windows left out instead of an error.
