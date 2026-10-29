use deskwright_core::ports::Clipboard;
use deskwright_core::{DeskwrightError, DeskwrightResult};
use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;

/// [`Clipboard`] over the general `NSPasteboard`, the one Cmd+C and Cmd+V use.
#[derive(Debug, Default)]
pub struct PasteboardClipboard;

impl PasteboardClipboard {
    pub fn new() -> Self {
        Self
    }
}

impl Clipboard for PasteboardClipboard {
    fn read_text(&self) -> DeskwrightResult<Option<String>> {
        Ok(autoreleasepool(|_| {
            // SAFETY: NSPasteboardTypeString is an immutable framework constant.
            let text_type = unsafe { NSPasteboardTypeString };
            NSPasteboard::generalPasteboard()
                .stringForType(text_type)
                .map(|text| text.to_string())
        }))
    }

    fn write_text(&self, text: &str) -> DeskwrightResult<()> {
        let was_written = autoreleasepool(|_| {
            let pasteboard = NSPasteboard::generalPasteboard();
            pasteboard.clearContents();
            // SAFETY: NSPasteboardTypeString is an immutable framework constant.
            let text_type = unsafe { NSPasteboardTypeString };
            pasteboard.setString_forType(&NSString::from_str(text), text_type)
        });
        if was_written {
            Ok(())
        } else {
            Err(DeskwrightError::operation_failed(
                "write the clipboard",
                "the pasteboard rejected the text",
            ))
        }
    }
}
