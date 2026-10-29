use deskwright_core::model::app::ProcessId;
use deskwright_core::ports::AppTerminator;
use deskwright_core::{DeskwrightError, DeskwrightResult};
use objc2::rc::autoreleasepool;
use objc2_app_kit::NSRunningApplication;

/// [`AppTerminator`] through `NSRunningApplication`: a polite quit request (the app's own Quit
/// handling, including save prompts) or a forced one.
#[derive(Debug, Default)]
pub struct RunningAppTerminator;

impl RunningAppTerminator {
    pub fn new() -> Self {
        Self
    }
}

impl AppTerminator for RunningAppTerminator {
    fn quit(&self, process_id: ProcessId, force: bool) -> DeskwrightResult<()> {
        let was_sent = autoreleasepool(|_| {
            NSRunningApplication::runningApplicationWithProcessIdentifier(process_id.0).map(|app| {
                if force {
                    app.forceTerminate()
                } else {
                    app.terminate()
                }
            })
        });
        match was_sent {
            Some(true) => Ok(()),
            Some(false) => Err(DeskwrightError::operation_failed(
                "quit the app",
                "macOS refused the request",
            )),
            None => Err(DeskwrightError::operation_failed(
                "quit the app",
                "it is no longer running",
            )),
        }
    }
}
