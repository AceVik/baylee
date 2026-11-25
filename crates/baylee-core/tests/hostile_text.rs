//! Text a stranger writes: deck rows, mana costs and printing facts. Each is
//! made of the pieces its reader looks for, in orders nobody would write, and
//! whatever is accepted has to mean the same once it is written back.

use baylee_core::card_metadata::RulesMetadata;
use baylee_core::deckrow::{self, MAX_COUNT, MAX_NOTE, Row, RowError};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::preset::Finish;

/// A fixed sequence (the 64-bit LCG of Knuth's MMIX), so a failure repeats.
struct Dice(u64);

impl Dice {
    fn roll(&mut self, sides: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap() % sides
    }

    fn pick<'a>(&mut self, words: &[&'a str]) -> &'a str {
        words[self.roll(words.len())]
    }
}

/// The row as it reads once written: the default finish is not written, and a
/// number without a set is not either (`Row`'s `Display`).
fn as_written(mut row: Row) -> Row {
    row.print.finish = row.print.finish.filter(|f| *f != Finish::Normal);
    if row.print.set.is_none() {
        row.print.collector_number = None;
    }
    row
}

const PIECES: [&str; 40] = [
    "Forest",
    "Æther",
    "Erase",
    "(Not",
    "One)",
    "(M11)",
    "(m11)",
    "(LEAX)",
    "(ab)",
    "()",
    "(",
    ")",
    "12",
    "12★",
    "WWK-105",
    "[de]",
    "[DE]",
    "[zhs]",
    "*F*",
    "*n*",
    "*foil*",
    "**",
    "*",
    "scryfall=x",
    "scryfall=",
    "circles=WU",
    "circles=UW",
    "#",
    "#note",
    "# (M11)",
    "1",
    "0",
    "-1",
    "a=b",
    "\t",
    "\u{a0}",
    "\u{202e}",
    "\u{1f600}",
    "é",
    "\"",
];

#[test]
fn a_row_that_is_read_reads_the_same_once_it_is_written_back() {
    let mut dice = Dice(1);
    let mut read = 0;
    for _ in 0..60_000 {
        let mut text = dice
            .pick(&["1", "4", "99", "+2", "1000000", "", "x"])
            .to_owned();
        for _ in 0..=dice.roll(6) {
            text.push_str(dice.pick(&[" ", " ", " ", "  "]));
            text.push_str(dice.pick(&PIECES));
        }
        let Ok(row) = deckrow::parse(&text) else {
            continue;
        };
        read += 1;
        assert!((1..=MAX_COUNT).contains(&row.count), "{text:?}");
        assert!(!row.name.is_empty(), "{text:?}");
        assert!(
            row.note
                .as_ref()
                .is_none_or(|n| n.chars().count() <= MAX_NOTE),
            "{text:?}"
        );
        let written = row.to_string();
        assert_eq!(
            deckrow::parse(&written).as_ref(),
            Ok(&as_written(row)),
            "{text:?} was written as {written:?}"
        );
    }
    assert!(read > 10_000, "the pieces make rows: {read}");
}

#[test]
fn a_count_is_from_one_to_its_bound() {
    let count = |text: &str| deckrow::parse(&format!("{text} Forest")).map(|row| row.count);
    assert_eq!(count("1"), Ok(1));
    assert_eq!(count("1000000"), Ok(MAX_COUNT));
    for bad in ["0", "1000001", "-1", "4294967296", "1.0", "1e3", "x", "٣"] {
        assert_eq!(count(bad), Err(RowError::Count), "{bad}");
    }
}

#[test]
fn a_group_said_twice_is_refused_and_one_alone_is_no_card() {
    for group in ["(M11)", "[de]", "*F*", "scryfall=x", "circles=WU"] {
        assert!(deckrow::parse(&format!("1 Forest {group}")).is_ok());
        assert!(
            deckrow::parse(&format!("1 Forest {group} {group}")).is_err(),
            "{group}"
        );
        assert_eq!(
            deckrow::parse(&format!("1 {group}")),
            Err(RowError::Shape),
            "{group}"
        );
    }
}

#[test]
fn a_note_is_bounded_in_characters_and_may_hold_anything() {
    let noted = |note: &str| deckrow::parse(&format!("1 Forest # {note}")).map(|row| row.note);
    let longest = "é".repeat(MAX_NOTE);
    assert_eq!(noted(&longest), Ok(Some(longest.clone())));
    assert_eq!(noted(&format!("{longest}é")), Err(RowError::Note));
    assert_eq!(noted("4 Island # x"), Ok(Some("4 Island # x".into())));
    assert_eq!(noted(""), Ok(None));
}

const SYMBOLS: [&str; 34] = [
    "{W}",
    "{U}",
    "{B}",
    "{R}",
    "{G}",
    "{C}",
    "{S}",
    "{X}",
    "{Y}",
    "{Z}",
    "{0}",
    "{1}",
    "{16}",
    "{4294967295}",
    "{4294967296}",
    "{W/U}",
    "{U/W}",
    "{2/W}",
    "{W/P}",
    "{W/U/P}",
    "{C/W}",
    "{H}",
    "{}",
    "{",
    "}",
    "{w}",
    "{W/}",
    "{/W}",
    "{W/W}",
    "{-1}",
    "W",
    "{½}",
    "{W U}",
    "{{W}}",
];

#[test]
fn a_cost_that_is_read_reads_the_same_once_it_is_written_back() {
    let mut dice = Dice(2);
    let mut read = 0;
    for _ in 0..60_000 {
        let mut text = String::new();
        for _ in 0..dice.roll(5) {
            // Mostly symbols that are ones, or nearly every text is refused.
            let among = if dice.roll(8) == 0 { SYMBOLS.len() } else { 21 };
            text.push_str(SYMBOLS[dice.roll(among)]);
        }
        let Ok(cost) = ManaCost::try_parse(&text) else {
            continue;
        };
        read += 1;
        let written = cost.to_string();
        let again = ManaCost::try_parse(&written).unwrap_or_else(|why| {
            panic!("{text:?} was written as {written:?}: {why}");
        });
        assert_eq!(again, cost, "{text:?} was written as {written:?}");
        assert_eq!((again.cmc(), again.len()), (cost.cmc(), cost.len()));
        // What the wire reads is what the reader reads.
        let json = serde_json::to_string(&cost).unwrap();
        assert_eq!(serde_json::from_str::<ManaCost>(&json).unwrap(), cost);
    }
    assert!(read > 10_000, "the symbols make costs: {read}");
}

#[test]
fn a_cost_far_past_any_card_s_saturates_and_is_read_without_an_end() {
    let generic = ManaCost::try_parse(&"{4294967295}".repeat(3)).expect("a cost");
    assert_eq!(
        (generic.generic_total(), generic.cmc()),
        (u32::MAX, u32::MAX)
    );
    let long = ManaCost::try_parse(&"{W}".repeat(70_000)).expect("a cost");
    assert_eq!(long.cmc(), u32::from(u16::MAX));
    assert_eq!(long.len(), u32::from(u16::MAX));
    let both =
        ManaCost::try_parse(&format!("{{4294967295}}{}", "{2/W}".repeat(9))).expect("a cost");
    assert_eq!(both.cmc(), u32::MAX);
}

#[test]
fn a_printing_s_watermarks_are_short_plain_words_on_at_least_one_face() {
    let mut rules = RulesMetadata::default();
    let card = CardIndex::new(1);
    let longest = "a".repeat(80);
    assert!(rules.insert(card, &[Some(&longest)], false).is_ok());
    let before = rules.clone();
    for bad in ["", "Azorius", "a b", "a_b", "é", "a\0", &"a".repeat(81)] {
        assert!(
            rules
                .insert(card, &[Some("izzet"), Some(bad)], false)
                .is_err(),
            "{bad:?}"
        );
    }
    assert!(rules.insert(card, &[], false).is_err());
    assert!(rules.insert(card, &[None; 256], false).is_err());
    assert_eq!(rules, before, "a refused printing leaves no word behind");
    assert!(rules.insert(card, &[None; 255], false).is_ok());
    assert!(rules.is_valid());
    rules.watermarks.push(longest);
    assert!(!rules.is_valid(), "one word under two numbers");
}
