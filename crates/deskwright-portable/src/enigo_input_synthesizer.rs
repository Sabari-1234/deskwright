use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use deskwright_core::model::geometry::Point;
use deskwright_core::model::key_chord::{ChordKey, KeyChord, Modifier, NamedKey};
use deskwright_core::polling::wait_until;
use deskwright_core::ports::InputSynthesizer;
use deskwright_core::{DeskwrightError, DeskwrightResult, Permission};
use enigo::{
    Button, Coordinate, Direction, Enigo, InputError, Key, Keyboard, Mouse, NewConError, Settings,
};

use crate::drag_path::drag_path;

/// How long a drag waits after pressing before moving, so macOS reports the button as held.
const DRAG_PRESS_SETTLE: Duration = Duration::from_millis(60);
/// Intermediate pointer positions between a drag's source and target.
const DRAG_STEP_COUNT: usize = 12;
const DRAG_STEP_PAUSE: Duration = Duration::from_millis(12);

/// [`InputSynthesizer`] backed by `enigo`.
///
/// The connection is opened on first use rather than at startup: `enigo` refuses to connect
/// until the input permission is granted, and the user may grant it while the server runs.
#[derive(Default)]
pub struct EnigoInputSynthesizer {
    connection: Mutex<Option<Enigo>>,
}

impl EnigoInputSynthesizer {
    pub fn new() -> Self {
        Self::default()
    }

    fn with_connection(
        &self,
        operation: &'static str,
        send_input: impl FnOnce(&mut Enigo) -> Result<(), InputError>,
    ) -> DeskwrightResult<()> {
        let mut connection = self.locked();
        let enigo = match &mut *connection {
            Some(open) => open,
            empty_slot @ None => empty_slot.insert(open_connection()?),
        };
        send_input(enigo)
            .map_err(|error| DeskwrightError::operation_failed(operation, error.to_string()))
    }

    // Input calls replace the whole connection or send self-contained events, so a panic
    // during one cannot leave shared state half-written; recovering from poisoning is safe.
    fn locked(&self) -> MutexGuard<'_, Option<Enigo>> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

fn open_connection() -> DeskwrightResult<Enigo> {
    let settings = Settings {
        // A system prompt would block a server that runs without a visible UI.
        open_prompt_to_get_permissions: false,
        ..Settings::default()
    };
    Enigo::new(&settings).map_err(connection_error)
}

fn connection_error(error: NewConError) -> DeskwrightError {
    match error {
        NewConError::NoPermission => DeskwrightError::PermissionDenied {
            permission: Permission::Accessibility,
        },
        other => DeskwrightError::operation_failed("open input connection", other.to_string()),
    }
}

/// How long to wait for the system to apply a mouse move before clicking anyway.
const CURSOR_ARRIVAL_TIMEOUT: Duration = Duration::from_millis(250);
const CURSOR_POLL_INTERVAL: Duration = Duration::from_millis(5);
/// Rounding between points and integer coordinates can leave the cursor one point off.
const CURSOR_POSITION_TOLERANCE: i32 = 1;

fn is_within_tolerance(actual: (i32, i32), target: (i32, i32)) -> bool {
    (actual.0 - target.0).abs() <= CURSOR_POSITION_TOLERANCE
        && (actual.1 - target.1).abs() <= CURSOR_POSITION_TOLERANCE
}

/// Moves the pointer and waits for the system to apply the move. enigo clicks at the cursor
/// position it reads back, and moves are applied asynchronously, so clicking at once would land
/// at the old position.
fn move_and_settle(enigo: &mut Enigo, point: Point) -> Result<(), InputError> {
    let target = (to_screen_pixel(point.x), to_screen_pixel(point.y));
    enigo.move_mouse(target.0, target.1, Coordinate::Abs)?;
    wait_until(CURSOR_ARRIVAL_TIMEOUT, CURSOR_POLL_INTERVAL, || {
        enigo
            .location()
            .is_ok_and(|location| is_within_tolerance(location, target))
    });
    Ok(())
}

fn to_screen_pixel(coordinate: f64) -> i32 {
    // Screen coordinates are far inside i32's range; the saturating cast is a safety net.
    coordinate.round() as i32
}

impl InputSynthesizer for EnigoInputSynthesizer {
    /// Sends one key event per character. `enigo`'s bulk text event is faster, but apps such
    /// as WhatsApp ignore it in search boxes (found in live testing), while per-key events
    /// behave exactly like a person typing.
    fn type_text(&self, text: &str) -> DeskwrightResult<()> {
        self.with_connection("type text", |enigo| {
            text.chars()
                .try_for_each(|character| enigo.key(typing_key(character), Direction::Click))
        })
    }

    fn click_at(&self, point: Point) -> DeskwrightResult<()> {
        self.with_connection("click", |enigo| {
            move_and_settle(enigo, point)?;
            enigo.button(Button::Left, Direction::Click)
        })
    }

    /// Two clicks in quick succession; enigo marks the second one as a double-click because it
    /// arrives within the system's double-click interval.
    fn double_click_at(&self, point: Point) -> DeskwrightResult<()> {
        self.with_connection("double-click", |enigo| {
            move_and_settle(enigo, point)?;
            enigo.button(Button::Left, Direction::Click)?;
            enigo.button(Button::Left, Direction::Click)
        })
    }

    fn right_click_at(&self, point: Point) -> DeskwrightResult<()> {
        self.with_connection("right-click", |enigo| {
            move_and_settle(enigo, point)?;
            enigo.button(Button::Right, Direction::Click)
        })
    }

    fn move_pointer_to(&self, point: Point) -> DeskwrightResult<()> {
        self.with_connection("move the pointer", |enigo| move_and_settle(enigo, point))
    }

    /// `enigo` sends "mouse dragged" rather than "mouse moved" events only once macOS reports
    /// the button as held, which lags the press; so it pauses after pressing, then moves in
    /// steps (live testing showed a single jump being treated as a click on the source).
    fn drag(&self, from: Point, to: Point) -> DeskwrightResult<()> {
        self.with_connection("drag", |enigo| {
            move_and_settle(enigo, from)?;
            enigo.button(Button::Left, Direction::Press)?;
            thread::sleep(DRAG_PRESS_SETTLE);
            let moved = drag_path(from, to, DRAG_STEP_COUNT)
                .into_iter()
                .try_for_each(|point| {
                    enigo.move_mouse(
                        to_screen_pixel(point.x),
                        to_screen_pixel(point.y),
                        Coordinate::Abs,
                    )?;
                    thread::sleep(DRAG_STEP_PAUSE);
                    Ok(())
                })
                .and_then(|()| move_and_settle(enigo, to));
            // Release even if the move failed, so the button never stays held down.
            let released = enigo.button(Button::Left, Direction::Release);
            moved.and(released)
        })
    }

    fn press_chord(&self, chord: &KeyChord) -> DeskwrightResult<()> {
        let modifier_keys: Vec<Key> = chord
            .modifiers()
            .iter()
            .copied()
            .map(modifier_key)
            .collect();
        self.with_connection("press keys", |enigo| {
            for &modifier in &modifier_keys {
                enigo.key(modifier, Direction::Press)?;
            }
            let tap_result = enigo.key(chord_key(chord.key()), Direction::Click);
            // Release every modifier even if the tap failed, so none stays held down.
            let release_result = modifier_keys
                .iter()
                .rev()
                .try_for_each(|&modifier| enigo.key(modifier, Direction::Release));
            tap_result.and(release_result)
        })
    }
}

fn typing_key(character: char) -> Key {
    match character {
        '\n' => Key::Return,
        '\t' => Key::Tab,
        other => Key::Unicode(other),
    }
}

fn modifier_key(modifier: Modifier) -> Key {
    match modifier {
        Modifier::Command => Key::Meta,
        Modifier::Control => Key::Control,
        Modifier::Option => Key::Alt,
        Modifier::Shift => Key::Shift,
    }
}

fn chord_key(key: ChordKey) -> Key {
    match key {
        ChordKey::Character(character) => Key::Unicode(character),
        ChordKey::Named(named) => match named {
            NamedKey::Enter => Key::Return,
            NamedKey::Escape => Key::Escape,
            NamedKey::Tab => Key::Tab,
            NamedKey::Space => Key::Space,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::UpArrow => Key::UpArrow,
            NamedKey::DownArrow => Key::DownArrow,
            NamedKey::LeftArrow => Key::LeftArrow,
            NamedKey::RightArrow => Key::RightArrow,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_permission_becomes_an_accessibility_permission_error() {
        assert_eq!(
            connection_error(NewConError::NoPermission),
            DeskwrightError::PermissionDenied {
                permission: Permission::Accessibility
            }
        );
    }

    #[test]
    fn other_connection_failures_keep_their_reason() {
        let error = connection_error(NewConError::EstablishCon("no event source"));

        assert!(
            error
                .to_string()
                .starts_with("open input connection failed:")
        );
    }

    #[test]
    fn types_newlines_and_tabs_as_their_keys() {
        assert_eq!(typing_key('\n'), Key::Return);
        assert_eq!(typing_key('\t'), Key::Tab);
        assert_eq!(typing_key('a'), Key::Unicode('a'));
        assert_eq!(typing_key('é'), Key::Unicode('é'));
    }

    #[test]
    fn maps_modifiers_to_the_portable_enigo_keys() {
        assert_eq!(modifier_key(Modifier::Command), Key::Meta);
        assert_eq!(modifier_key(Modifier::Option), Key::Alt);
        assert_eq!(modifier_key(Modifier::Control), Key::Control);
        assert_eq!(modifier_key(Modifier::Shift), Key::Shift);
    }

    #[test]
    fn maps_named_keys_and_characters() {
        assert_eq!(chord_key(ChordKey::Named(NamedKey::Enter)), Key::Return);
        assert_eq!(
            chord_key(ChordKey::Named(NamedKey::PageDown)),
            Key::PageDown
        );
        assert_eq!(chord_key(ChordKey::Character('g')), Key::Unicode('g'));
    }

    #[test]
    fn cursor_one_point_off_counts_as_arrived() {
        assert!(is_within_tolerance((101, 49), (100, 50)));
        assert!(!is_within_tolerance((102, 50), (100, 50)));
    }

    #[test]
    fn rounds_coordinates_to_the_nearest_pixel() {
        assert_eq!(to_screen_pixel(10.4), 10);
        assert_eq!(to_screen_pixel(10.5), 11);
        assert_eq!(to_screen_pixel(-3.6), -4);
    }
}
