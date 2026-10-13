//! Received text is cleaned before it is drawn (§10.7), and a body's spans survive only an untouched text.

use crate::social::convs::{Body, BodyLink, BodyMention};
use crate::social::text::{self, MAX_LINE_BREAKS};

fn link(start: usize, end: usize) -> BodyLink {
    BodyLink {
        at: [start, end],
        card: 7,
        face: 0,
        print: None,
    }
}

fn mention(start: usize, end: usize, tag: &str) -> BodyMention {
    BodyMention {
        at: [start, end],
        tag: tag.to_string(),
    }
}

fn body(text: &str, links: Vec<BodyLink>, mentions: Vec<BodyMention>) -> Body {
    Body {
        text: text.to_string(),
        links,
        mentions,
        emote: false,
    }
}

#[test]
fn clean_leaves_ordinary_text_alone() {
    let text = "Grüße aus Köln 🙂 — « ok »\nnext line";
    assert_eq!(text::clean(text), text);
    assert!(!text::needs_cleaning(text));
}

#[test]
fn clean_drops_c0_controls_but_keeps_the_line_feed() {
    assert_eq!(
        text::clean("a\u{0000}b\u{0007}c\u{001B}d\te\rf\u{007F}g\nh"),
        "abcdefg\nh"
    );
}

#[test]
fn clean_drops_c1_controls() {
    assert_eq!(text::clean("a\u{0080}b\u{0085}c\u{009F}d"), "abcd");
}

#[test]
fn clean_drops_every_bidi_control() {
    for c in ('\u{202A}'..='\u{202E}').chain('\u{2066}'..='\u{2069}') {
        assert_eq!(text::clean(&format!("a{c}b")), "ab", "U+{:04X}", c as u32);
    }
    for c in ['\u{200E}', '\u{200F}', '\u{061C}'] {
        assert_eq!(text::clean(&format!("a{c}b")), "ab", "U+{:04X}", c as u32);
    }
}

#[test]
fn clean_drops_zero_width_characters_and_the_byte_order_mark() {
    for c in [
        '\u{200B}', '\u{200C}', '\u{200D}', '\u{2060}', '\u{2064}', '\u{FEFF}', '\u{00AD}',
    ] {
        assert_eq!(text::clean(&format!("a{c}b")), "ab", "U+{:04X}", c as u32);
    }
}

#[test]
fn clean_caps_at_one_thousand_characters_not_bytes() {
    let long: String = "ä".repeat(1500);
    let cleaned = text::clean(&long);
    assert_eq!(cleaned.chars().count(), 1000);
    assert!(cleaned.chars().all(|c| c == 'ä'));
    assert_eq!(text::clean(&"x".repeat(1000)).chars().count(), 1000);
}

#[test]
fn clean_does_not_count_dropped_characters_against_the_cap() {
    let text = format!("{}{}", "\u{200B}".repeat(500), "a".repeat(1000));
    assert_eq!(text::clean(&text), "a".repeat(1000));
}

#[test]
fn clean_keeps_six_line_breaks_and_turns_later_ones_into_spaces() {
    assert_eq!(MAX_LINE_BREAKS, 6);
    let text = "a\n".repeat(9);
    let cleaned = text::clean(&text);
    assert_eq!(cleaned.matches('\n').count(), 6);
    assert_eq!(cleaned, "a\na\na\na\na\na\na a a ");
    assert_eq!(cleaned.chars().count(), text.chars().count());
}

#[test]
fn clean_name_drops_line_breaks_and_hidden_characters() {
    assert_eq!(
        text::clean_name("Al\nice\r\u{202E}#af\u{200B}03"),
        "Alice#af03"
    );
}

#[test]
fn clean_name_caps_at_sixty_four_characters() {
    assert_eq!(text::clean_name(&"é".repeat(100)).chars().count(), 64);
    assert_eq!(text::clean_name(&"é".repeat(64)).chars().count(), 64);
}

#[test]
fn needs_cleaning_says_whether_clean_would_change_the_text() {
    assert!(text::needs_cleaning("a\u{202E}b"));
    assert!(text::needs_cleaning(&"a".repeat(1001)));
    assert!(text::needs_cleaning(&"\n".repeat(7)));
    assert!(!text::needs_cleaning(&"\n".repeat(6)));
}

#[test]
fn checked_keeps_spans_inside_an_untouched_text() {
    let b = body(
        "see [Bolt] and [@Bob#0012]",
        vec![link(4, 10)],
        vec![mention(15, 26, "0012")],
    )
    .checked()
    .expect("a body");
    assert_eq!(b.links, vec![link(4, 10)]);
    assert_eq!(b.mentions, vec![mention(15, 26, "0012")]);
    assert_eq!(b.span_text([4, 10]), "[Bolt]");
    assert_eq!(b.span_text([15, 26]), "[@Bob#0012]");
}

#[test]
fn checked_drops_spans_past_the_end_of_the_text() {
    let b = body("hello", vec![link(2, 6), link(10, 12)], Vec::new())
        .checked()
        .expect("a body");
    assert!(b.links.is_empty());
}

#[test]
fn checked_keeps_a_span_that_ends_exactly_at_the_end() {
    let b = body("hello", vec![link(0, 5)], Vec::new())
        .checked()
        .expect("a body");
    assert_eq!(b.links, vec![link(0, 5)]);
}

#[test]
fn checked_counts_span_ends_in_characters_not_bytes() {
    // Eight characters, twelve bytes: a byte-counting check would accept [0, 10].
    let b = body("ääää [x]", vec![link(0, 10), link(5, 8)], Vec::new())
        .checked()
        .expect("a body");
    assert_eq!(b.links, vec![link(5, 8)]);
}

#[test]
fn checked_drops_an_empty_or_reversed_span() {
    let b = body("hello", vec![link(3, 3), link(4, 2)], Vec::new())
        .checked()
        .expect("a body");
    assert!(b.links.is_empty());
}

#[test]
fn checked_drops_every_span_when_two_overlap() {
    let b = body(
        "[Bolt][@Bob#0012] tail",
        vec![link(0, 8)],
        vec![mention(6, 17, "0012")],
    )
    .checked()
    .expect("a body");
    assert!(b.links.is_empty());
    assert!(b.mentions.is_empty());
}

#[test]
fn checked_accepts_spans_that_touch_without_overlapping() {
    let b = body(
        "[Bolt][@Bob#0012]",
        vec![link(0, 6)],
        vec![mention(6, 17, "0012")],
    )
    .checked()
    .expect("a body");
    assert_eq!(b.links.len(), 1);
    assert_eq!(b.mentions.len(), 1);
}

#[test]
fn checked_sorts_spans_by_position() {
    let b = body(
        "[a] [b] [c]",
        vec![link(8, 11), link(0, 3), link(4, 7)],
        Vec::new(),
    )
    .checked()
    .expect("a body");
    let starts: Vec<usize> = b.links.iter().map(|l| l.at[0]).collect();
    assert_eq!(starts, [0, 4, 8]);
}

#[test]
fn checked_drops_all_spans_of_a_text_that_needed_cleaning() {
    for (case, text) in [
        "[Bolt]\u{202E} x".to_string(),
        format!("[Bolt] {}", "a".repeat(1100)),
        format!("[Bolt] {}", "\n".repeat(8)),
    ]
    .into_iter()
    .enumerate()
    {
        let b = body(&text, vec![link(0, 6)], vec![mention(0, 6, "0012")])
            .checked()
            .expect("a body");
        assert!(b.links.is_empty(), "links of case {case}");
        assert!(b.mentions.is_empty());
        assert!(!text::needs_cleaning(&b.text), "the text is the clean one");
    }
}

#[test]
fn checked_cleans_the_text_it_returns() {
    let b = body("he\u{202E}llo", Vec::new(), Vec::new())
        .checked()
        .expect("a body");
    assert_eq!(b.text, "hello");
}

#[test]
fn checked_turns_an_all_whitespace_body_into_none() {
    for text in ["", "   ", "\n\n", " \t\u{200B}\u{202E} \n", "\u{200B}"] {
        assert!(
            body(text, vec![link(0, 1)], Vec::new()).checked().is_none(),
            "{text:?}"
        );
    }
}

#[test]
fn checked_drops_mentions_with_an_invalid_tag() {
    let tags = ["", "abc", "00123", "ABCD", "zzzz", "123456789", "0012 "];
    for tag in tags {
        let b = body("[@Bob#0012] hi", Vec::new(), vec![mention(0, 11, tag)])
            .checked()
            .expect("a body");
        assert!(b.mentions.is_empty(), "tag {tag:?}");
    }
    for tag in ["0012", "af03", "1234abcd", "10000"] {
        let b = body("[@Bob#0012] hi", Vec::new(), vec![mention(0, 11, tag)])
            .checked()
            .expect("a body");
        assert_eq!(b.mentions.len(), 1, "tag {tag:?}");
    }
}

#[test]
fn checked_keeps_the_emote_flag() {
    let mut b = body("waves", Vec::new(), Vec::new());
    b.emote = true;
    assert!(b.checked().expect("a body").emote);
}

#[test]
fn mentions_tag_finds_only_mentioned_tags() {
    let b = body("[@Bob#0012]", Vec::new(), vec![mention(0, 11, "0012")]);
    assert!(b.mentions_tag("0012"));
    assert!(!b.mentions_tag("af03"));
}
