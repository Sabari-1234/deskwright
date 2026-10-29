use crate::error::DeskwrightResult;
use crate::model::geometry::Point;
use crate::model::key_chord::KeyChord;

/// Generates synthetic keyboard and mouse input, as if the user typed or clicked.
pub trait InputSynthesizer: Send + Sync {
    /// Types text into whatever currently has keyboard focus.
    fn type_text(&self, text: &str) -> DeskwrightResult<()>;

    fn click_at(&self, point: Point) -> DeskwrightResult<()>;

    fn double_click_at(&self, point: Point) -> DeskwrightResult<()>;

    fn right_click_at(&self, point: Point) -> DeskwrightResult<()>;

    /// Moves the pointer without clicking, as when hovering.
    fn move_pointer_to(&self, point: Point) -> DeskwrightResult<()>;

    /// Presses the primary button at `from`, moves to `to`, and releases there.
    fn drag(&self, from: Point, to: Point) -> DeskwrightResult<()>;

    /// Presses the chord's modifiers, taps its key, then releases the modifiers. The keys go
    /// to whichever app is frontmost.
    fn press_chord(&self, chord: &KeyChord) -> DeskwrightResult<()>;
}
