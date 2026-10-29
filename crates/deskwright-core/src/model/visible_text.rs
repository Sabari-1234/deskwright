/// Unicode bidirectional marks that some apps (WhatsApp) wrap around labels and even their
/// app name; they render as nothing.
const INVISIBLE_DIRECTION_MARKS: [char; 6] = [
    '\u{200e}', '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}',
];

/// The text a person sees: invisible direction marks removed, surrounding space trimmed.
pub fn visible_text(text: &str) -> String {
    text.chars()
        .filter(|character| !INVISIBLE_DIRECTION_MARKS.contains(character))
        .collect::<String>()
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_direction_marks_and_surrounding_space() {
        assert_eq!(visible_text("\u{200e}WhatsApp "), "WhatsApp");
        assert_eq!(visible_text("\u{202a}Ada Lovelace\u{202c}"), "Ada Lovelace");
        assert_eq!(visible_text("plain"), "plain");
    }
}
