use std::fmt;

use crate::error::{DeskwrightError, DeskwrightResult};

const CHORD_SEPARATOR: char = '+';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Modifier {
    Command,
    Control,
    Option,
    Shift,
}

impl Modifier {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "cmd" | "command" | "⌘" | "meta" => Some(Self::Command),
            "ctrl" | "control" | "⌃" => Some(Self::Control),
            "opt" | "option" | "alt" | "⌥" => Some(Self::Option),
            "shift" | "⇧" => Some(Self::Shift),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Command => "Cmd",
            Self::Control => "Ctrl",
            Self::Option => "Option",
            Self::Shift => "Shift",
        }
    }
}

/// A key with a name, as opposed to a key that types a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedKey {
    Enter,
    Escape,
    Tab,
    Space,
    Backspace,
    Delete,
    UpArrow,
    DownArrow,
    LeftArrow,
    RightArrow,
    Home,
    End,
    PageUp,
    PageDown,
}

impl NamedKey {
    const ALL: [Self; 14] = [
        Self::Enter,
        Self::Escape,
        Self::Tab,
        Self::Space,
        Self::Backspace,
        Self::Delete,
        Self::UpArrow,
        Self::DownArrow,
        Self::LeftArrow,
        Self::RightArrow,
        Self::Home,
        Self::End,
        Self::PageUp,
        Self::PageDown,
    ];

    fn parse(name: &str) -> Option<Self> {
        match name {
            "enter" | "return" => Some(Self::Enter),
            "esc" | "escape" => Some(Self::Escape),
            "up" => Some(Self::UpArrow),
            "down" => Some(Self::DownArrow),
            "left" => Some(Self::LeftArrow),
            "right" => Some(Self::RightArrow),
            other => Self::ALL
                .into_iter()
                .find(|key| key.label().eq_ignore_ascii_case(other)),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Enter => "Enter",
            Self::Escape => "Escape",
            Self::Tab => "Tab",
            Self::Space => "Space",
            Self::Backspace => "Backspace",
            Self::Delete => "Delete",
            Self::UpArrow => "UpArrow",
            Self::DownArrow => "DownArrow",
            Self::LeftArrow => "LeftArrow",
            Self::RightArrow => "RightArrow",
            Self::Home => "Home",
            Self::End => "End",
            Self::PageUp => "PageUp",
            Self::PageDown => "PageDown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordKey {
    Named(NamedKey),
    /// A key that types this character; letters are stored lowercase.
    Character(char),
}

/// A key pressed together with zero or more modifiers, such as `Cmd+Shift+G` or `Enter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyChord {
    modifiers: Vec<Modifier>,
    key: ChordKey,
}

impl KeyChord {
    /// Parses names separated by `+`, ignoring case: modifiers first, the key last.
    pub fn parse(raw: &str) -> DeskwrightResult<Self> {
        let names: Vec<String> = raw
            .split(CHORD_SEPARATOR)
            .map(|name| name.trim().to_lowercase())
            .collect();
        let Some((key_name, modifier_names)) = names.split_last() else {
            return Err(invalid_chord(raw, "it is empty"));
        };
        let mut modifiers = modifier_names
            .iter()
            .map(|name| {
                Modifier::parse(name)
                    .ok_or_else(|| invalid_chord(raw, &format!("'{name}' is not a modifier")))
            })
            .collect::<DeskwrightResult<Vec<_>>>()?;
        modifiers.sort_unstable();
        modifiers.dedup();
        let key = parse_key(key_name).ok_or_else(|| {
            invalid_chord(
                raw,
                &format!("'{key_name}' is not a key name or a single character"),
            )
        })?;
        Ok(Self { modifiers, key })
    }

    pub fn modifiers(&self) -> &[Modifier] {
        &self.modifiers
    }

    pub fn key(&self) -> ChordKey {
        self.key
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for modifier in &self.modifiers {
            write!(formatter, "{}{CHORD_SEPARATOR}", modifier.label())?;
        }
        match self.key {
            ChordKey::Named(named) => formatter.write_str(named.label()),
            ChordKey::Character(character) => {
                write!(formatter, "{}", character.to_uppercase())
            }
        }
    }
}

fn parse_key(name: &str) -> Option<ChordKey> {
    if let Some(named) = NamedKey::parse(name) {
        return Some(ChordKey::Named(named));
    }
    let mut characters = name.chars();
    match (characters.next(), characters.next()) {
        (Some(character), None) if !character.is_control() => Some(ChordKey::Character(character)),
        _ => None,
    }
}

fn invalid_chord(raw: &str, reason: &str) -> DeskwrightError {
    DeskwrightError::InvalidInput(format!(
        "'{raw}' is not a key chord ({reason}); use forms like 'Enter', 'Escape' or 'Cmd+Shift+G'"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_named_key_alone() {
        let chord = KeyChord::parse("Enter").unwrap();

        assert!(chord.modifiers().is_empty());
        assert_eq!(chord.key(), ChordKey::Named(NamedKey::Enter));
    }

    #[test]
    fn accepts_aliases_for_keys_and_modifiers_ignoring_case() {
        assert_eq!(
            KeyChord::parse("return").unwrap().key(),
            ChordKey::Named(NamedKey::Enter)
        );
        assert_eq!(
            KeyChord::parse("ESC").unwrap().key(),
            ChordKey::Named(NamedKey::Escape)
        );
        assert_eq!(
            KeyChord::parse("ctrl+opt+down").unwrap().modifiers(),
            [Modifier::Control, Modifier::Option]
        );
        assert_eq!(
            KeyChord::parse("pageup").unwrap().key(),
            ChordKey::Named(NamedKey::PageUp)
        );
    }

    #[test]
    fn parses_modifiers_with_a_character_key_in_a_stable_order() {
        let chord = KeyChord::parse(" shift + Cmd + G ").unwrap();

        assert_eq!(chord.modifiers(), [Modifier::Command, Modifier::Shift]);
        assert_eq!(chord.key(), ChordKey::Character('g'));
        assert_eq!(chord.to_string(), "Cmd+Shift+G");
    }

    #[test]
    fn ignores_repeated_modifiers() {
        assert_eq!(
            KeyChord::parse("cmd+command+s").unwrap().modifiers(),
            [Modifier::Command]
        );
    }

    #[test]
    fn displays_named_keys_by_their_label() {
        assert_eq!(
            KeyChord::parse("alt+left").unwrap().to_string(),
            "Option+LeftArrow"
        );
    }

    #[test]
    fn rejects_empty_unknown_and_multi_character_keys() {
        for invalid in ["", "cmd+", "hyper+s", "cmd+save", "f13"] {
            assert!(KeyChord::parse(invalid).is_err(), "{invalid}");
        }
    }
}
