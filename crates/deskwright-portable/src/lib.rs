//! Adapters for deskwright-core ports built on cross-platform libraries, so every desktop backend
//! (macOS today, Windows later) can reuse them.

mod display_region;
mod drag_path;
mod enigo_input_synthesizer;
mod image_encoding;
mod window_match;
mod xcap_screen_capturer;

pub use enigo_input_synthesizer::EnigoInputSynthesizer;
pub use xcap_screen_capturer::XcapScreenCapturer;
