//! What a selection copies and what a double or triple click selects (§10.5).

use crate::social::convs::{ConvId, LineKey};
use crate::social::select::{DrawnLine, Pos, Selection, copy_text, line_at, word_at};

fn key(id: &str) -> LineKey {
    LineKey {
        conv: ConvId::Table { game: "g1".into() },
        id: id.to_string(),
    }
}

fn pos(id: &str, byte: usize) -> Pos {
    Pos {
        line: key(id),
        byte,
    }
}

fn sel(a: (&str, usize), f: (&str, usize)) -> Selection {
    Selection {
        anchor: pos(a.0, a.1),
        focus: pos(f.0, f.1),
    }
}

/// Three lines: Alice, Bob, and a notice with neither clock nor name.
fn drawn<'a>(keys: &'a [LineKey; 3], texts: [&'a str; 3]) -> [DrawnLine<'a>; 3] {
    [
        DrawnLine {
            key: &keys[0],
            clock: "09:01",
            name: "Alice",
            text: texts[0],
        },
        DrawnLine {
            key: &keys[1],
            clock: "09:02",
            name: "Bob",
            text: texts[1],
        },
        DrawnLine {
            key: &keys[2],
            clock: "09:03",
            name: "Alice",
            text: texts[2],
        },
    ]
}

fn keys() -> [LineKey; 3] {
    [key("a"), key("b"), key("c")]
}

#[test]
fn a_selection_inside_one_line_copies_just_those_characters() {
    let keys = keys();
    let lines = drawn(&keys, ["first line", "second", "third"]);
    assert_eq!(copy_text(&lines, &sel(("a", 6), ("a", 10))), "line");
    assert_eq!(copy_text(&lines, &sel(("b", 0), ("b", 3))), "sec");
}

#[test]
fn a_selection_inside_one_line_has_no_clock_or_name() {
    let keys = keys();
    let lines = drawn(&keys, ["first line", "second", "third"]);
    let copied = copy_text(&lines, &sel(("a", 0), ("a", 10)));
    assert_eq!(copied, "first line");
}

#[test]
fn a_selection_dragged_backwards_inside_one_line_copies_the_same() {
    let keys = keys();
    let lines = drawn(&keys, ["first line", "second", "third"]);
    assert_eq!(copy_text(&lines, &sel(("a", 10), ("a", 6))), "line");
}

#[test]
fn an_empty_selection_copies_nothing() {
    let keys = keys();
    let lines = drawn(&keys, ["first line", "second", "third"]);
    assert_eq!(copy_text(&lines, &sel(("a", 4), ("a", 4))), "");
}

#[test]
fn a_selection_over_several_lines_copies_each_as_clock_name_text() {
    let keys = keys();
    let lines = drawn(&keys, ["first line", "second", "third"]);
    // From the start of the first line to the end of the last.
    let copied = copy_text(&lines, &sel(("a", 0), ("c", 5)));
    assert_eq!(
        copied,
        "09:01 Alice: first line\n09:02 Bob: second\n09:03 Alice: third"
    );
}

#[test]
fn only_the_selected_part_of_the_first_and_last_line_is_copied() {
    let keys = keys();
    let lines = drawn(&keys, ["first line", "second", "third"]);
    let copied = copy_text(&lines, &sel(("a", 6), ("c", 3)));
    assert_eq!(
        copied,
        "09:01 Alice: line\n09:02 Bob: second\n09:03 Alice: thi"
    );
}

#[test]
fn a_selection_over_two_adjacent_lines_has_no_middle() {
    let keys = keys();
    let lines = drawn(&keys, ["first line", "second", "third"]);
    let copied = copy_text(&lines, &sel(("a", 6), ("b", 3)));
    assert_eq!(copied, "09:01 Alice: line\n09:02 Bob: sec");
}

#[test]
fn a_selection_dragged_upwards_copies_what_one_dragged_downwards_does() {
    let keys = keys();
    let lines = drawn(&keys, ["first line", "second", "third"]);
    let down = copy_text(&lines, &sel(("a", 6), ("c", 3)));
    let up = copy_text(&lines, &sel(("c", 3), ("a", 6)));
    assert_eq!(up, down);
}

#[test]
fn a_last_line_selected_from_its_very_start_is_not_copied() {
    let keys = keys();
    let lines = drawn(&keys, ["first line", "second", "third"]);
    let copied = copy_text(&lines, &sel(("a", 0), ("c", 0)));
    assert_eq!(copied, "09:01 Alice: first line\n09:02 Bob: second");
}

#[test]
fn empty_clock_and_name_are_left_out() {
    let keys = keys();
    let mut lines = drawn(&keys, ["first line", "a notice", "third"]);
    lines[1].clock = "";
    lines[1].name = "";
    lines[2].clock = "";
    let copied = copy_text(&lines, &sel(("a", 0), ("c", 5)));
    assert_eq!(copied, "09:01 Alice: first line\na notice\nAlice: third");
}

#[test]
fn with_timestamps_off_every_clock_is_left_out_of_a_partial_first_line_too() {
    let keys = keys();
    let mut lines = drawn(&keys, ["first line", "second", "third"]);
    for line in &mut lines {
        line.clock = "";
    }
    let copied = copy_text(&lines, &sel(("a", 6), ("b", 6)));
    assert_eq!(copied, "Alice: line\nBob: second");
}

#[test]
fn a_links_bracket_text_is_copied_as_it_is_drawn() {
    let keys = keys();
    let lines = drawn(&keys, ["cast [Lightning Bolt] now", "ok", "x"]);
    let whole = copy_text(&lines, &sel(("a", 0), ("b", 2)));
    assert_eq!(
        whole,
        "09:01 Alice: cast [Lightning Bolt] now\n09:02 Bob: ok"
    );
    let inside = copy_text(&lines, &sel(("a", 5), ("a", 21)));
    assert_eq!(inside, "[Lightning Bolt]");
}

#[test]
fn a_byte_inside_a_character_moves_back_to_its_start() {
    let keys = keys();
    let lines = drawn(&keys, ["Jörg 🙂 hi", "b", "c"]);
    // 'ö' is bytes 1..3; the emoji is bytes 6..10; 2 and 8 fall inside them.
    assert_eq!(copy_text(&lines, &sel(("a", 2), ("a", 8))), "örg ");
    assert_eq!(copy_text(&lines, &sel(("a", 0), ("a", 100))), "Jörg 🙂 hi");
}

#[test]
fn a_selection_naming_a_line_that_is_not_drawn_copies_nothing() {
    let keys = keys();
    let lines = drawn(&keys, ["first line", "second", "third"]);
    assert_eq!(copy_text(&lines, &sel(("a", 0), ("zzz", 2))), "");
    assert_eq!(copy_text(&lines, &sel(("zzz", 0), ("a", 2))), "");
    assert_eq!(copy_text(&[], &sel(("a", 0), ("a", 2))), "");
}

#[test]
fn double_click_selects_a_word() {
    let text = "hello brave world";
    for byte in [6, 8, 10] {
        assert_eq!(word_at(text, byte), 6..11, "byte {byte}");
    }
    assert_eq!(word_at(text, 0), 0..5);
    assert_eq!(word_at(text, 16), 12..17);
}

#[test]
fn double_click_on_a_name_with_a_tag_selects_the_whole_handle() {
    let text = "hi Bob#0012, ok";
    for byte in [3, 5, 6, 10] {
        assert_eq!(word_at(text, byte), 3..11, "byte {byte}");
    }
    assert_eq!(&text[word_at(text, 7)], "Bob#0012");
}

#[test]
fn double_click_on_punctuation_or_a_space_selects_that_one_character() {
    let text = "hi Bob#0012, ok";
    assert_eq!(word_at(text, 11), 11..12);
    assert_eq!(word_at(text, 12), 12..13);
    assert_eq!(word_at("a (b)", 2), 2..3);
}

#[test]
fn double_click_keeps_hyphens_underscores_and_apostrophes_in_a_word() {
    assert_eq!(word_at("a well-known fact", 4), 2..12);
    assert_eq!(word_at("my_name here", 1), 0..7);
    assert_eq!(word_at("it's fine", 0), 0..4);
    assert_eq!(word_at("it’s fine", 0), 0..6);
}

#[test]
fn double_click_selects_a_multibyte_word_whole() {
    let text = "grüße dich 日本語 x";
    // "grüße" is 7 bytes; byte 3 is inside the 'ü'.
    assert_eq!(word_at(text, 3), 0..7);
    assert_eq!(word_at(text, 2), 0..7);
    assert_eq!(word_at(text, 4), 0..7);
    let cjk = text.find('日').expect("a cjk word");
    assert_eq!(&text[word_at(text, cjk + 4)], "日本語");
}

#[test]
fn double_click_on_an_emoji_selects_the_emoji() {
    let text = "a 🙂 b";
    assert_eq!(word_at(text, 2), 2..6);
    assert_eq!(word_at(text, 4), 2..6);
}

#[test]
fn double_click_past_the_end_selects_nothing() {
    assert_eq!(word_at("hello", 5), 5..5);
    assert_eq!(word_at("hello", 50), 5..5);
    assert_eq!(word_at("", 0), 0..0);
}

#[test]
fn triple_click_selects_the_whole_line() {
    assert_eq!(line_at("hello brave world"), 0..17);
    assert_eq!(line_at("Jörg 🙂"), 0.."Jörg 🙂".len());
    assert_eq!(line_at(""), 0..0);
}
