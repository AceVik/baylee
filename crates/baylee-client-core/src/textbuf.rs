//! One text field's contents, caret and selection.
//!
//! The client has two text platforms and must not grow two text models. In a
//! browser the authority is a real `<input>` — that is what buys autofill, an
//! IME, a phone keyboard and paste, and none of it is worth reimplementing —
//! so there the buffer is a *mirror*: [`TextBuffer::set`] takes the value and
//! the caret the element reports and the canvas draws them. On the desktop
//! there is no element, so the same buffer is the authority and the key
//! handler drives it through the same methods. Either way the lobby's
//! headless tests answer like a player, because there is one model under
//! both.
//!
//! What it deliberately is not: a text *editor*. One line, no wrapping, no
//! undo, no clipboard of its own. `Line` movement is Home and End on a field
//! that has exactly one line, and it is a granularity rather than two more
//! methods so a key handler can pass what the key meant and stop caring.
//!
//! Offsets are byte offsets into the string and always sit on a `char`
//! boundary, which is what lets the caret be handed to and taken from a
//! browser's `selectionStart` unchanged. They are counted in `char`s and not
//! in grapheme clusters: a cluster-aware caret needs a table this crate does
//! not carry, and the fields it serves are an address, a display name and a
//! password.

use std::ops::Range;

/// How far one movement reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// One character — the arrow keys.
    Char,
    /// One word — the arrow keys with the platform's word modifier.
    Word,
    /// The whole line, which is the whole field — Home and End.
    Line,
}

/// Which way a movement goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    /// Towards the start of the text.
    Left,
    /// Towards the end of it.
    Right,
}

/// A single-line text field's contents, caret and selection.
///
/// The caret is `cursor`. `anchor` is the other end of a selection and is
/// `None` when there is none; it is never `Some` and equal to `cursor`, so
/// [`TextBuffer::selection`] answering `Some` always means a non-empty run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextBuffer {
    text: String,
    cursor: usize,
    anchor: Option<usize>,
}

impl TextBuffer {
    /// A buffer holding `text`, with the caret after it and nothing selected.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            cursor: text.len(),
            text: text.to_string(),
            anchor: None,
        }
    }

    /// What the field holds.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Where the caret is, as a byte offset.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The selected run, if any. Never empty.
    #[must_use]
    pub fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let (lo, hi) = (anchor.min(self.cursor), anchor.max(self.cursor));
        (lo < hi).then_some(lo..hi)
    }

    /// Whether the field is empty — what a placeholder is drawn for.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Empties it and takes the caret home.
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.anchor = None;
    }

    /// Replaces contents and caret wholesale — the browser's `<input>` said
    /// the value is now this and the caret is there.
    ///
    /// Both offsets are clamped onto a `char` boundary rather than trusted,
    /// because they arrive from outside this crate: a caret in the middle of
    /// a multi-byte character would panic the next time anything sliced the
    /// string at it, and a field that ate an umlaut is not the place to find
    /// that out.
    pub fn set(&mut self, text: &str, cursor: usize, anchor: Option<usize>) {
        self.text = text.to_string();
        self.place(cursor, anchor);
    }

    /// Moves the caret and the selection without touching the text.
    ///
    /// The other half of [`TextBuffer::set`], for the same mirror: an arrow
    /// key inside a browser's own `<input>` moves that element's caret and
    /// changes nothing else, and a caret drawn from a value that did not
    /// change would sit still while the real one walked away from it.
    ///
    /// Clamped onto `char` boundaries exactly as `set` is, and for the same
    /// reason.
    pub fn place(&mut self, cursor: usize, anchor: Option<usize>) {
        self.cursor = self.boundary(cursor);
        self.anchor = anchor
            .map(|a| self.boundary(a))
            .filter(|a| *a != self.cursor);
    }

    /// Selects everything. A no-op on an empty field, which keeps the
    /// invariant that a selection is never empty.
    pub fn select_all(&mut self) {
        if self.text.is_empty() {
            return;
        }
        self.anchor = Some(0);
        self.cursor = self.text.len();
    }

    /// Drops the selection, leaving the caret where it is.
    pub fn deselect(&mut self) {
        self.anchor = None;
    }

    /// Inserts text at the caret, replacing the selection if there is one.
    ///
    /// Control characters are dropped rather than inserted: Enter submits the
    /// form and Tab moves the caret, so both reach this in some shells as a
    /// character, and a field holding a newline is a field that draws one.
    pub fn insert(&mut self, s: &str) {
        let cleaned: String = s.chars().filter(|c| !c.is_control()).collect();
        self.replace_selection(&cleaned);
    }

    /// Replaces the selection — or, with nothing selected, inserts at the
    /// caret. Takes `s` as given, controls and all; [`TextBuffer::insert`] is
    /// the door typed text comes through.
    pub fn replace_selection(&mut self, s: &str) {
        let range = self.selection().unwrap_or(self.cursor..self.cursor);
        let at = range.start;
        self.text.replace_range(range, s);
        self.cursor = at + s.len();
        self.anchor = None;
    }

    /// Backspace: the selection, or the character before the caret.
    pub fn delete_back(&mut self) {
        if self.selection().is_some() {
            self.replace_selection("");
            return;
        }
        let Some(prev) = self.prev_char(self.cursor) else {
            return;
        };
        self.text.replace_range(prev..self.cursor, "");
        self.cursor = prev;
    }

    /// Delete: the selection, or the character after the caret.
    pub fn delete_forward(&mut self) {
        if self.selection().is_some() {
            self.replace_selection("");
            return;
        }
        let Some(next) = self.next_char(self.cursor) else {
            return;
        };
        self.text.replace_range(self.cursor..next, "");
    }

    /// Moves the caret, extending the selection when `select` is set.
    ///
    /// Without `select` and with something selected, a plain arrow collapses
    /// to the near end of the selection rather than moving off the far one —
    /// which is what every text field does and what a caret that jumped a
    /// character past the run it just deselected would not.
    pub fn move_caret(&mut self, step: Step, dir: Dir, select: bool) {
        if !select && let Some(sel) = self.selection() {
            self.anchor = None;
            if step == Step::Char {
                self.cursor = match dir {
                    Dir::Left => sel.start,
                    Dir::Right => sel.end,
                };
                return;
            }
        }
        if select {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = self.target(step, dir);
        if self.anchor == Some(self.cursor) {
            self.anchor = None;
        }
    }

    /// Where a movement lands, without moving anything.
    fn target(&self, step: Step, dir: Dir) -> usize {
        match (step, dir) {
            (Step::Line, Dir::Left) => 0,
            (Step::Line, Dir::Right) => self.text.len(),
            (Step::Char, Dir::Left) => self.prev_char(self.cursor).unwrap_or(0),
            (Step::Char, Dir::Right) => self.next_char(self.cursor).unwrap_or(self.text.len()),
            (Step::Word, Dir::Left) => self.word_left(),
            (Step::Word, Dir::Right) => self.word_right(),
        }
    }

    /// The start of the word before the caret: the whitespace immediately
    /// behind it is skipped first, so a caret sitting after a run of spaces
    /// reaches the word rather than the gap.
    fn word_left(&self) -> usize {
        let mut at = self.cursor;
        while let Some(prev) = self.prev_char(at) {
            if !self.char_at(prev).is_whitespace() {
                break;
            }
            at = prev;
        }
        while let Some(prev) = self.prev_char(at) {
            if self.char_at(prev).is_whitespace() {
                break;
            }
            at = prev;
        }
        at
    }

    /// The end of the word after the caret, by the same rule read forwards.
    fn word_right(&self) -> usize {
        let mut at = self.cursor;
        while at < self.text.len() && self.char_at(at).is_whitespace() {
            at = self.next_char(at).unwrap_or(self.text.len());
        }
        while at < self.text.len() && !self.char_at(at).is_whitespace() {
            at = self.next_char(at).unwrap_or(self.text.len());
        }
        at
    }

    /// The character starting at `at`, which every caller has already
    /// established is a boundary inside the string.
    fn char_at(&self, at: usize) -> char {
        self.text[at..].chars().next().unwrap_or('\0')
    }

    /// The boundary before `at`, or `None` at the start of the text.
    fn prev_char(&self, at: usize) -> Option<usize> {
        self.text[..at]
            .chars()
            .next_back()
            .map(|c| at - c.len_utf8())
    }

    /// The boundary after `at`, or `None` at the end of it.
    fn next_char(&self, at: usize) -> Option<usize> {
        self.text[at..].chars().next().map(|c| at + c.len_utf8())
    }

    /// The nearest `char` boundary at or before `at`, clamped to the text.
    fn boundary(&self, at: usize) -> usize {
        let mut at = at.min(self.text.len());
        while at > 0 && !self.text.is_char_boundary(at) {
            at -= 1;
        }
        at
    }
}

/// Where a UTF-16 offset falls in a Rust string, as a byte offset.
///
/// The one conversion between this module and a browser: the DOM counts an
/// `<input>`'s `selectionStart` in UTF-16 code units and everything here
/// counts bytes, and the two agree for exactly as long as a player types
/// ASCII. An address with an umlaut in it is where they stop agreeing, and a
/// caret that is off by one from the character it is drawn beside deletes the
/// wrong letter.
///
/// An offset past the end answers the end, and one that falls *inside* a
/// surrogate pair — which no selection a browser reports should ever be —
/// answers the boundary after that character rather than a byte offset no
/// `char` starts at.
#[must_use]
pub fn byte_of_utf16(text: &str, units: usize) -> usize {
    let mut seen = 0;
    for (byte, ch) in text.char_indices() {
        if seen >= units {
            return byte;
        }
        seen += ch.len_utf16();
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_buffer_holds_its_text_with_the_caret_after_it() {
        let buf = TextBuffer::new("mail@example.com");
        assert_eq!(buf.text(), "mail@example.com");
        assert_eq!(buf.cursor(), 16);
        assert_eq!(buf.selection(), None);
    }

    #[test]
    fn typing_lands_at_the_caret_and_not_at_the_end() {
        let mut buf = TextBuffer::new("ac");
        buf.move_caret(Step::Char, Dir::Left, false);
        buf.insert("b");
        assert_eq!(buf.text(), "abc");
        assert_eq!(buf.cursor(), 2);
    }

    #[test]
    fn a_control_character_is_not_a_character() {
        let mut buf = TextBuffer::new("");
        buf.insert("a\nb\tc");
        assert_eq!(buf.text(), "abc");
    }

    #[test]
    fn backspace_and_delete_take_one_character_each() {
        let mut buf = TextBuffer::new("abc");
        buf.delete_back();
        assert_eq!(buf.text(), "ab");
        buf.move_caret(Step::Line, Dir::Left, false);
        buf.delete_forward();
        assert_eq!(buf.text(), "b");
        assert_eq!(buf.cursor(), 0);
    }

    #[test]
    fn deleting_at_either_end_does_nothing_at_all() {
        let mut buf = TextBuffer::new("a");
        buf.move_caret(Step::Line, Dir::Left, false);
        buf.delete_back();
        buf.move_caret(Step::Line, Dir::Right, false);
        buf.delete_forward();
        assert_eq!(buf.text(), "a");
    }

    #[test]
    fn a_multi_byte_character_is_one_character_to_every_movement() {
        let mut buf = TextBuffer::new("mär");
        buf.move_caret(Step::Char, Dir::Left, false);
        assert_eq!(buf.cursor(), 3, "before the r, which is after ä");
        buf.move_caret(Step::Char, Dir::Left, false);
        assert_eq!(buf.cursor(), 1, "before the ä, which is two bytes wide");
        buf.delete_forward();
        assert_eq!(buf.text(), "mr");
    }

    #[test]
    fn selecting_and_typing_replaces_the_run() {
        let mut buf = TextBuffer::new("secret");
        buf.select_all();
        assert_eq!(buf.selection(), Some(0..6));
        buf.insert("x");
        assert_eq!(buf.text(), "x");
        assert_eq!(buf.selection(), None, "the selection is spent");
        assert_eq!(buf.cursor(), 1);
    }

    #[test]
    fn select_all_on_an_empty_field_selects_nothing() {
        let mut buf = TextBuffer::new("");
        buf.select_all();
        assert_eq!(buf.selection(), None);
    }

    #[test]
    fn backspace_over_a_selection_takes_the_whole_run() {
        let mut buf = TextBuffer::new("abcdef");
        buf.move_caret(Step::Line, Dir::Left, false);
        buf.move_caret(Step::Char, Dir::Right, true);
        buf.move_caret(Step::Char, Dir::Right, true);
        assert_eq!(buf.selection(), Some(0..2));
        buf.delete_back();
        assert_eq!(buf.text(), "cdef");
        assert_eq!(buf.cursor(), 0);
    }

    #[test]
    fn shrinking_a_selection_back_onto_its_anchor_ends_it() {
        let mut buf = TextBuffer::new("abc");
        buf.move_caret(Step::Char, Dir::Left, true);
        assert_eq!(buf.selection(), Some(2..3));
        buf.move_caret(Step::Char, Dir::Right, true);
        assert_eq!(buf.selection(), None, "the two ends met");
    }

    #[test]
    fn a_plain_arrow_collapses_to_the_near_end_of_a_selection() {
        let mut buf = TextBuffer::new("abcdef");
        buf.select_all();
        buf.move_caret(Step::Char, Dir::Left, false);
        assert_eq!(buf.cursor(), 0);
        assert_eq!(buf.selection(), None);
        buf.select_all();
        buf.move_caret(Step::Char, Dir::Right, false);
        assert_eq!(buf.cursor(), 6);
    }

    #[test]
    fn word_movement_skips_the_gap_and_stops_at_the_word() {
        let mut buf = TextBuffer::new("one two  three");
        buf.move_caret(Step::Word, Dir::Left, false);
        assert_eq!(&buf.text()[buf.cursor()..], "three");
        buf.move_caret(Step::Word, Dir::Left, false);
        assert_eq!(&buf.text()[buf.cursor()..], "two  three");
        buf.move_caret(Step::Word, Dir::Right, false);
        assert_eq!(&buf.text()[buf.cursor()..], "  three");
        buf.move_caret(Step::Word, Dir::Right, false);
        assert_eq!(buf.cursor(), buf.text().len());
    }

    #[test]
    fn word_movement_at_the_ends_stays_inside_the_text() {
        let mut buf = TextBuffer::new("  ");
        buf.move_caret(Step::Word, Dir::Left, false);
        assert_eq!(buf.cursor(), 0);
        buf.move_caret(Step::Word, Dir::Right, false);
        assert_eq!(buf.cursor(), 2);
    }

    #[test]
    fn the_browser_may_say_where_the_caret_is() {
        let mut buf = TextBuffer::new("");
        buf.set("filled by the password manager", 6, Some(0));
        assert_eq!(buf.text(), "filled by the password manager");
        assert_eq!(buf.cursor(), 6);
        assert_eq!(buf.selection(), Some(0..6));
    }

    #[test]
    fn a_caret_offered_off_a_boundary_is_pulled_onto_one() {
        let mut buf = TextBuffer::new("");
        buf.set("mär", 2, None);
        assert_eq!(buf.cursor(), 1, "inside the ä, so back to before it");
        buf.set("mär", 99, None);
        assert_eq!(buf.cursor(), 4, "past the end, so the end");
    }

    #[test]
    fn an_anchor_that_lands_on_the_caret_is_no_selection() {
        let mut buf = TextBuffer::new("");
        buf.set("abc", 3, Some(3));
        assert_eq!(buf.selection(), None);
    }

    #[test]
    fn a_caret_can_move_without_the_text_changing() {
        let mut buf = TextBuffer::new("mail@example.com");
        buf.place(4, Some(0));
        assert_eq!(buf.text(), "mail@example.com", "place writes no text");
        assert_eq!(buf.cursor(), 4);
        assert_eq!(buf.selection(), Some(0..4));
        buf.place(2, None);
        assert_eq!(buf.selection(), None, "an anchor of None drops it");
    }

    #[test]
    fn a_placed_caret_is_clamped_like_a_set_one() {
        let mut buf = TextBuffer::new("mär");
        buf.place(2, Some(99));
        assert_eq!(buf.cursor(), 1, "inside the a-umlaut, so back to before it");
        assert_eq!(buf.selection(), Some(1..4), "and the anchor to the end");
    }

    #[test]
    fn a_utf16_offset_is_read_as_bytes() {
        assert_eq!(byte_of_utf16("abc", 0), 0);
        assert_eq!(
            byte_of_utf16("abc", 2),
            2,
            "ASCII counts the same either way"
        );
        // "mär": m is one unit and one byte, a-umlaut is one unit and two.
        assert_eq!(byte_of_utf16("mär", 1), 1);
        assert_eq!(byte_of_utf16("mär", 2), 3, "after the a-umlaut");
        assert_eq!(byte_of_utf16("mär", 3), 4);
    }

    #[test]
    fn an_astral_character_is_two_units_and_four_bytes() {
        let text = "a🜁b";
        assert_eq!(byte_of_utf16(text, 1), 1, "before the sigil");
        assert_eq!(
            byte_of_utf16(text, 3),
            5,
            "after it: 1 + 2 units, 1 + 4 bytes"
        );
        assert_eq!(
            byte_of_utf16(text, 2),
            5,
            "halfway through a surrogate pair is no caret at all, so the far side"
        );
        assert_eq!(
            byte_of_utf16(text, 99),
            text.len(),
            "past the end is the end"
        );
    }
}
