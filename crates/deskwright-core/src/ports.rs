//! Port traits: what the core needs from the outside world. Platform adapters implement
//! them; services depend only on these abstractions. Each trait is deliberately small so an
//! adapter never has to stub out methods it cannot support.

pub mod accessibility_permission;
pub mod app_directory;
pub mod app_launcher;
pub mod app_terminator;
pub mod clipboard;
pub mod element_actions;
pub mod element_tree_reader;
pub mod hit_tester;
pub mod input_synthesizer;
pub mod menu_bar_locator;
pub mod screen_capturer;
pub mod script_runner;
pub mod window_controller;

pub use accessibility_permission::AccessibilityPermission;
pub use app_directory::AppDirectory;
pub use app_launcher::AppLauncher;
pub use app_terminator::AppTerminator;
pub use clipboard::Clipboard;
pub use element_actions::ElementActions;
pub use element_tree_reader::ElementTreeReader;
pub use hit_tester::HitTester;
pub use input_synthesizer::InputSynthesizer;
pub use menu_bar_locator::MenuBarLocator;
pub use screen_capturer::ScreenCapturer;
pub use script_runner::ScriptRunner;
pub use window_controller::WindowController;
