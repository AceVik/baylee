//! Every externally supplied byte offset must remain safe through real edits.
use super::*;

const SAMPLES: &[&str] = &[
    "",
    "a",
    "äöü",
    "a😀中z",
    "e\u{301} café",
    "👨\u{200d}👩\u{200d}👧",
    "אבג xyz",
];

fn assert_valid(buffer: &TextBuffer) {
    assert!(buffer.text().is_char_boundary(buffer.cursor()));
    if let Some(range) = buffer.selection() {
        assert!(range.start < range.end);
        assert!(buffer.text().is_char_boundary(range.start));
        assert!(buffer.text().is_char_boundary(range.end));
    }
    let segments = buffer.segments();
    assert_eq!(
        format!("{}{}{}", segments.head, segments.selected, segments.tail),
        buffer.text()
    );
}

#[test]
fn every_byte_offset_and_selection_survives_editing_unicode() {
    for text in SAMPLES {
        for cursor in (0..=text.len() + 1).chain([usize::MAX]) {
            for anchor in (0..=text.len() + 1)
                .map(Some)
                .chain([None, Some(usize::MAX)])
            {
                let mut original = TextBuffer::default();
                original.set(text, cursor, anchor);
                assert_valid(&original);
                for direction in [Dir::Left, Dir::Right] {
                    for step in [Step::Char, Step::Word, Step::Line] {
                        for select in [true, false] {
                            let mut moved = original.clone();
                            moved.move_caret(step, direction, select);
                            assert_valid(&moved);
                            moved.insert("😀é");
                            assert_valid(&moved);
                            moved.delete_back();
                            assert_valid(&moved);
                            moved.delete_forward();
                            assert_valid(&moved);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn reverse_selections_replace_the_same_unicode_run() {
    for (cursor, anchor) in [(1, 7), (7, 1)] {
        let mut buffer = TextBuffer::default();
        buffer.set("a😀éz", cursor, Some(anchor));
        assert_eq!(buffer.selection(), Some(1..7));
        assert_eq!(buffer.segments().selected, "😀é");
        buffer.insert("中");
        assert_eq!(buffer.text(), "a中z");
        assert_eq!(buffer.cursor(), 4);
        assert_eq!(buffer.selection(), None);
        assert_valid(&buffer);
    }
}

#[test]
fn repeated_character_deletion_removes_scalars_without_splitting_utf8() {
    for text in SAMPLES {
        let chars: Vec<_> = text.chars().collect();
        let mut back = TextBuffer::new(text);
        let mut front = TextBuffer::new(text);
        front.move_caret(Step::Line, Dir::Left, false);
        for removed in 1..=chars.len() {
            back.delete_back();
            front.delete_forward();
            assert_eq!(
                back.text(),
                chars[..chars.len() - removed].iter().collect::<String>()
            );
            assert_eq!(front.text(), chars[removed..].iter().collect::<String>());
            assert_valid(&back);
            assert_valid(&front);
        }
        back.delete_back();
        front.delete_forward();
        assert!(back.is_empty() && front.is_empty());
    }
}

#[test]
fn clear_resets_selection_and_accepts_new_unicode_input() {
    let mut buffer = TextBuffer::new("😀旧内容");
    buffer.select_all();
    buffer.clear();
    assert_eq!(buffer.cursor(), 0);
    assert_eq!(buffer.selection(), None);
    buffer.insert("neu中");
    assert_eq!(buffer.text(), "neu中");
    assert_eq!(buffer.cursor(), 6);
    assert_valid(&buffer);
}

#[test]
fn discarded_control_input_does_not_erase_a_selection() {
    for input in ["", "\n", "\t", "\r\n", "\0\u{7f}\u{85}"] {
        let mut buffer = TextBuffer::new("selected😀");
        buffer.select_all();
        let before = buffer.clone();
        buffer.insert(input);
        assert_eq!(buffer, before, "ignored input must not act as Delete");
    }
}

#[test]
fn mixed_control_and_visible_input_replaces_the_selection_once() {
    let mut buffer = TextBuffer::new("old😀");
    buffer.select_all();
    buffer.insert("\n中\t😀\r");
    assert_eq!(buffer.text(), "中😀");
    assert_eq!(buffer.cursor(), 7);
    assert_eq!(buffer.selection(), None);
}

#[test]
fn explicit_empty_replacement_still_deletes_selected_text() {
    let mut buffer = TextBuffer::new("old😀");
    buffer.select_all();
    buffer.replace_selection("");
    assert!(buffer.is_empty());
    assert_eq!(buffer.cursor(), 0);
    assert_eq!(buffer.selection(), None);
}
