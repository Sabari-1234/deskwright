# Screenshots

> Captures one element, one window, or the whole main display, at point or pixel
> resolution, as PNG or JPEG.

**Status:** Implemented
**Last updated:** 2026-10-29

## Purpose

Some content isn't in the accessibility tree: images, canvases, video, games. Screenshots
cover those cases. Snapshots stay the main way to understand UI because they're faster
and structured.

## How it works

1. `ScreenshotTool` maps its arguments to a `ScreenshotTarget` and `CaptureOptions`:
   - `ref` → `Element`; `app` with optional `title`/`index` → `Window`; nothing →
     `MainDisplay`. Giving both a ref and a window is an error.
   - `resolution`: `"points"` (default) or `"pixels"`; `format`: `"png"` (default) or
     `"jpeg"` with `quality` 1–100 (default 80). `quality` with PNG is an error.
2. `ScreenshotService` resolves the target:
   - an element's on-screen bounds (`ElementTreeReader::on_screen_bounds`), rejecting
     elements with no visible area;
   - a window through `WindowService::find`, rejecting minimized and zero-size windows.
3. `XcapScreenCapturer` captures:
   - the display, or a region clipped to the display under its center (`DisplayRegion`);
   - a window from **its own pixels** (`xcap::Window`), matched by process and frame
     (`window_match`), so the AI app's window in front of it doesn't appear. A window that
     isn't on screen (another Space, or hidden by Stage Manager) is refused with a hint to
     raise it, rather than capturing whatever else is shown at its position.
4. `image_encoding::encode` scales a Retina capture down to the area's size in points (a
   quarter of the pixels), unless `resolution` is `"pixels"`, then encodes PNG or JPEG.
5. The tool returns an MCP image block with the right MIME type (`image/png` or
   `image/jpeg`).

## Measurements

On a 3024×1964 Retina display (M-series Mac), measured with `deskwright run`:

| Capture | Time | Size |
|---|---|---|
| Full display, pixels, PNG (the old default) | 44 ms | 842 KB |
| Full display, points, PNG (default) | 47 ms | 387 KB |
| Full display, points, JPEG | 55 ms | 150 KB |
| 900×560 window, points, JPEG | 174 ms | 65 KB |
| 900×560 window, pixels, PNG | 143 ms | 583 KB |

Capturing and encoding take tens of milliseconds; what costs time in an AI client is
moving and reading the image, so the smaller defaults matter most. Window captures spend
most of their time listing windows.

## Design decisions

| Decision | Reason | Alternatives considered |
|---|---|---|
| Point resolution by default | Matches Playwright's default `scale: "css"`; a quarter of the pixels on Retina and still readable | Full pixels by default (the first version) |
| JPEG as an option, PNG as the default | PNG keeps small text exact; JPEG is 2.5–6× smaller when that matters | JPEG by default |
| Window capture from the window's own pixels | The AI app's window is usually in front of the target app | Cropping the screen to the window's bounds |
| Refuse off-screen windows | A wrong picture (another app) is worse than a clear error | Falling back to the screen area (found to capture the wrong app under Stage Manager) |
| Scaling and encoding in a pure module | Testable without Screen Recording permission | Inside the `xcap` adapter |
| `xcap` in the cross-platform `deskwright-portable` crate | Works on macOS, Windows and Linux, so every backend reuses it | Shelling out to `screencapture` |

## Files involved

| Layer | File | Responsibility |
|---|---|---|
| Domain | `crates/deskwright-core/src/model/encoded_image.rs` | `ImageFormat`, `CaptureResolution`, `CaptureOptions`, `EncodedImage` |
| Domain | `crates/deskwright-core/src/ports/screen_capturer.rs` | Display, region and window capture |
| Application | `crates/deskwright-core/src/services/screenshot_service.rs` | Target resolution |
| Application | `crates/deskwright-core/src/services/window_service.rs` | `find` for window targets |
| Adapter | `crates/deskwright-portable/src/xcap_screen_capturer.rs` | Thin calls into `xcap` |
| Adapter | `crates/deskwright-portable/src/image_encoding.rs` | Scaling to points, PNG and JPEG |
| Adapter | `crates/deskwright-portable/src/window_match.rs` | Matching a window by process and frame |
| Adapter | `crates/deskwright-portable/src/display_region.rs` | Clipping a region to one display |
| Interface | `crates/deskwright-mcp/src/tools/screenshot_tool.rs` | `desk_screenshot` |

## Inputs and outputs

```json
{ "tool": "desk_screenshot", "params": {} }
{ "tool": "desk_screenshot", "params": { "app": "Finder", "format": "jpeg" } }
{ "tool": "desk_screenshot", "params": { "ref": "e12", "resolution": "pixels" } }
```

Result: one image content block. Errors: unknown ref, element or window without a visible
area, minimized or off-screen window, invalid quality, capture failure (usually a missing
Screen Recording permission).

## How to extend

A new format is an `ImageFormat` variant and a branch in `image_encoding::encode`; a new
platform's capturer implements the three `ScreenCapturer` methods.

## Testing

`encoded_image.rs` tests quality validation, MIME types and defaults. `image_encoding.rs`
tests halving a Retina image to points, keeping pixels when asked, never upscaling, and
JPEG output. `window_match.rs` tests matching by process and frame within rounding.
`screenshot_service.rs` tests element and window targets, options pass-through, and refusing
minimized or unsized windows. `crates/deskwright-mcp/tests/window_and_app_tools_tests.rs`
tests the tool's arguments and errors, and `element_tools_tests.rs` the element and display
targets. The `xcap` calls need Screen Recording permission and are excluded from the
coverage gate (see `docs/TESTING.md`).

## Limitations and future work

- Under Stage Manager, only windows on the current stage can be captured; raising a window
  with `desk_window` does not always move it onto the stage.
- When two processes share an app name (two Chrome instances), the first is used; give its
  bundle ID or raise the right one first.
- An element that spans two displays is captured only on the display under its center.
- Without the Screen Recording permission, macOS may return an image with other apps'
  windows left out instead of an error.
