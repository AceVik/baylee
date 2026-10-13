//! Selecting and copying the history (§10.5, §11.10): the pure part. The
//! renderer hit-tests glyphs into a [`Pos`]; this decides what a drag, a
//! double-click or a triple-click selects and what Copy puts on the
//! clipboard.

use std::ops::Range;

use super::convs::LineKey;

/// A place in the history: a line and a byte offset into its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pos {
    /// The line.
    pub line: LineKey,
    /// A byte offset into the line's text, on a character boundary.
    pub byte: usize,
}

/// A selection: where the press began and where it is now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    /// Where it began.
    pub anchor: Pos,
    /// Where it is now.
    pub focus: Pos,
}

/// One line as drawn, which Copy reproduces: `HH:MM Name: text`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawnLine<'a> {
    /// The line.
    pub key: &'a LineKey,
    /// Its clock, `HH:MM`, or `""` with timestamps off.
    pub clock: &'a str,
    /// Its sender as drawn, or `""` for a notice.
    pub name: &'a str,
    /// Its text as drawn (links as their bracket text).
    pub text: &'a str,
}

impl DrawnLine<'_> {
    fn whole(&self) -> String {
        let mut out = String::new();
        if !self.clock.is_empty() {
            out.push_str(self.clock);
            out.push(' ');
        }
        if !self.name.is_empty() {
            out.push_str(self.name);
            out.push_str(": ");
        }
        out.push_str(self.text);
        out
    }

    fn whole_from(&self, byte: usize) -> String {
        let mut out = String::new();
        if byte == 0 {
            return self.whole();
        }
        if !self.clock.is_empty() {
            out.push_str(self.clock);
            out.push(' ');
        }
        if !self.name.is_empty() {
            out.push_str(self.name);
            out.push_str(": ");
        }
        out.push_str(&self.text[clamp(self.text, byte)..]);
        out
    }
}

/// What a selection copies, given the lines as drawn, in order: inside one
/// line, the selected characters; across lines, each line as
/// `HH:MM Name: text` (only the selected part of the first and the last),
/// one per row.
#[must_use]
pub fn copy_text(lines: &[DrawnLine<'_>], selection: &Selection) -> String {
    let index = |pos: &Pos| lines.iter().position(|l| *l.key == pos.line);
    let (Some(a), Some(f)) = (index(&selection.anchor), index(&selection.focus)) else {
        return String::new();
    };
    let ((first, first_byte), (last, last_byte)) =
        if (a, selection.anchor.byte) <= (f, selection.focus.byte) {
            ((a, selection.anchor.byte), (f, selection.focus.byte))
        } else {
            ((f, selection.focus.byte), (a, selection.anchor.byte))
        };
    if first == last {
        let text = lines[first].text;
        return text[clamp(text, first_byte)..clamp(text, last_byte)].to_string();
    }
    let mut rows = Vec::with_capacity(last - first + 1);
    rows.push(lines[first].whole_from(first_byte));
    for line in &lines[first + 1..last] {
        rows.push(line.whole());
    }
    let end = lines[last];
    let mut tail = DrawnLine {
        text: &end.text[..clamp(end.text, last_byte)],
        ..end
    }
    .whole();
    if last_byte == 0 {
        tail.clear();
    }
    if !tail.is_empty() {
        rows.push(tail);
    }
    rows.join("\n")
}

/// The word under `byte` (double-click): a run of letters, digits and the
/// characters names are made of, or the single other character there.
#[must_use]
pub fn word_at(text: &str, byte: usize) -> Range<usize> {
    let byte = clamp(text, byte);
    let in_word = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | '\'' | '’' | '#');
    let Some(here) = text[byte..].chars().next() else {
        return byte..byte;
    };
    if !in_word(here) {
        return byte..byte + here.len_utf8();
    }
    let start = text[..byte]
        .char_indices()
        .rev()
        .take_while(|(_, c)| in_word(*c))
        .last()
        .map_or(byte, |(i, _)| i);
    let end = text[byte..]
        .char_indices()
        .find(|(_, c)| !in_word(*c))
        .map_or(text.len(), |(i, _)| byte + i);
    start..end
}

/// The whole line (triple-click).
#[must_use]
pub fn line_at(text: &str) -> Range<usize> {
    0..text.len()
}

/// `byte` moved back onto a character boundary inside `text`.
fn clamp(text: &str, byte: usize) -> usize {
    let mut at = byte.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}
