//! What arrives is cleaned before it is drawn (§10.7): another client's text
//! may carry characters that draw nothing, reorder what follows them, or
//! stack line after line.

use baylee_protocol::social::MAX_TEXT_CHARS;

/// The most line breaks a received text keeps; later ones become spaces.
pub const MAX_LINE_BREAKS: usize = 6;

/// A control character but `\n`, or a format character that draws nothing
/// or turns the direction of what follows it: the C0 and C1 controls, the
/// bidi embeddings, overrides, isolates and marks, the Arabic letter mark,
/// zero-width characters, word joiners and the byte-order mark.
#[must_use]
pub fn is_hidden(c: char) -> bool {
    (c.is_control() && c != '\n')
        || matches!(
            c,
            '\u{00AD}'
                | '\u{061C}'
                | '\u{180E}'
                | '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{206F}'
                | '\u{FEFF}'
                | '\u{FFF9}'..='\u{FFFB}'
        )
}

/// A received message's text as it may be drawn: hidden characters dropped
/// ([`is_hidden`]), at most [`MAX_LINE_BREAKS`] line breaks (later ones
/// become spaces) and at most [`MAX_TEXT_CHARS`] characters.
#[must_use]
pub fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(MAX_TEXT_CHARS * 4));
    let mut breaks = 0;
    for c in text.chars().filter(|c| !is_hidden(*c)).take(MAX_TEXT_CHARS) {
        if c == '\n' {
            breaks += 1;
            out.push(if breaks > MAX_LINE_BREAKS { ' ' } else { '\n' });
        } else {
            out.push(c);
        }
    }
    out
}

/// A handle or a name from the gateway, as drawn: hidden characters and
/// every line break dropped, at most 64 characters.
#[must_use]
pub fn clean_name(name: &str) -> String {
    name.chars()
        .filter(|c| !is_hidden(*c) && *c != '\n')
        .take(64)
        .collect()
}

/// Whether [`clean`] would change `text`: a received body that needs
/// cleaning had its spans counted against other characters, so its links
/// and mentions are dropped rather than drawn in the wrong places.
#[must_use]
pub fn needs_cleaning(text: &str) -> bool {
    clean(text) != text
}
