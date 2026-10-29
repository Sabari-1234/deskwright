//! macOS adapters for deskwright-core ports: the Accessibility API (tree reading, element actions,
//! windows, menu bar, permission), running apps (listing and quitting), the clipboard, and
//! AppleScript/JXA execution.
#![cfg(target_os = "macos")]

mod accessibility;
mod open_command_app_launcher;
mod osascript_script_runner;
mod pasteboard_clipboard;
mod process_list;
mod running_app_terminator;
mod workspace_app_directory;

pub use accessibility::{
    AxAccessibilityPermission, AxElementActions, AxHitTester, AxMenuBarLocator, AxTreeReader,
    AxWindowController, configure_accessibility_timeout,
};
pub use open_command_app_launcher::OpenCommandAppLauncher;
pub use osascript_script_runner::OsaScriptRunner;
pub use pasteboard_clipboard::PasteboardClipboard;
pub use running_app_terminator::RunningAppTerminator;
pub use workspace_app_directory::WorkspaceAppDirectory;
