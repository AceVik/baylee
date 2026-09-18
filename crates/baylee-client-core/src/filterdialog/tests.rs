//! The dialog's round trip, held against the language's own corpus.
//!
//! One property carries this module and the rest of it is detail:
//! decomposing a query and recomposing it gives back *the same query*. It is
//! asserted over [`crate::cardquery::tests::CORPUS`] rather than over cases,
//! and deliberately over that corpus and not one of its own — the corpus is
//! one line per shape the grammar makes, so a grammar that grows a shape the
//! dialog cannot carry fails here on the same day it is added, instead of on
//! the day a player opens the gear on a line that uses it.

use super::{Control, FilterForm, FilterPart};
use crate::cardquery::tests::CORPUS;
use crate::cardquery::{Colors, Key, Op, Surface, Term, Value, parse, render};

#[test]
fn a_form_puts_back_exactly_what_it_took_apart() {
    for line in CORPUS {
        let query = parse(line);
        let back = FilterForm::of(&query).query();
        assert_eq!(back, query, "`{line}` came back as `{}`", render(&back));
    }
}

/// Opening the dialog and closing it writes the same string back.
///
/// The property above is about trees; this is the one a player sees. They
/// differ in exactly one place — a string that is not written canonically —
/// so the comparison is against the *rendered* query and not against what
/// was typed.
#[test]
fn opening_the_dialog_and_closing_it_changes_nothing() {
    for line in CORPUS {
        let once = render(&parse(line));
        let after = render(&FilterForm::of(&parse(line)).query());
        assert_eq!(after, once, "`{line}` was rewritten by being looked at");
    }
}

#[test]
fn a_flat_line_is_all_controls_and_nothing_carried() {
    let form = FilterForm::of(&parse("t:creature -c:w mv<=3"));
    assert_eq!(form.parts.len(), 3);
    assert!(
        form.parts
            .iter()
            .all(|part| matches!(part, FilterPart::Row { .. })),
        "every part of a flat conjunction has a control"
    );
    let FilterPart::Row { negated, term } = &form.parts[1] else {
        panic!("the second part is a row");
    };
    assert!(*negated, "the minus travels with the term, not beside it");
    assert_eq!(term.key, Key::Color);
}

/// A branch keeps its place, which is what makes the round trip exact.
///
/// Pushing what the controls cannot draw to the end would be the ordinary
/// way to write this, and it is the reason most filter builders rewrite the
/// line they were opened on. Here the parts stay in the order they were
/// written, so the rebuilt query is the one that came in.
#[test]
fn what_no_control_can_draw_keeps_its_place() {
    let query = parse("t:creature (c:r or c:g) mv:3");
    let form = FilterForm::of(&query);
    assert!(matches!(form.parts[0], FilterPart::Row { .. }));
    assert!(matches!(form.parts[1], FilterPart::Opaque(_)));
    assert!(matches!(form.parts[2], FilterPart::Row { .. }));
    assert_eq!(render(&form.query()), "t:creature (c:r or c:g) mv:3");
}

/// A line the controls cannot take apart at all is carried whole.
#[test]
fn an_or_at_the_top_is_one_chip_and_no_controls() {
    let form = FilterForm::of(&parse("t:fish or t:bird"));
    assert_eq!(form.parts.len(), 1);
    assert!(matches!(form.parts[0], FilterPart::Opaque(_)));
    assert_eq!(render(&form.query()), "t:fish or t:bird");
}

#[test]
fn an_empty_box_opens_an_empty_dialog() {
    let form = FilterForm::of(&parse(""));
    assert!(form.parts.is_empty(), "no term, so no row");
    assert!(
        form.query().is_anything(),
        "and closing it leaves the box empty rather than writing brackets"
    );
}

#[test]
fn a_control_writes_over_its_own_row_and_appends_a_new_one() {
    let mut form = FilterForm::of(&parse("t:creature mv:3"));
    form.set(&Key::ManaValue, Op::Le, Value::Number(5), false);
    assert_eq!(
        render(&form.query()),
        "t:creature mv<=5",
        "the mana value control rewrote the term that was already there"
    );
    form.set(&Key::Color, Op::Colon, Value::Colors(Colors::R), true);
    assert_eq!(render(&form.query()), "t:creature mv<=5 -c:r");
    assert!(form.clear(&Key::ManaValue));
    assert_eq!(render(&form.query()), "t:creature -c:r");
    assert!(
        !form.clear(&Key::ManaValue),
        "clearing what is not there says so rather than pretending"
    );
}

/// Two terms with the same key are two conditions, and stay two.
///
/// `mv>=2 mv<=4` is an ordinary range and `t:creature t:goblin` an ordinary
/// pair of types. A form of named fields would have held one of each and
/// dropped the other on the way in; a control writes over the **first** row
/// for its key and leaves the second where the player put it.
#[test]
fn a_second_term_with_the_same_key_is_not_the_same_control() {
    let mut form = FilterForm::of(&parse("mv>=2 mv<=4"));
    assert_eq!(form.parts.len(), 2);
    form.set(&Key::ManaValue, Op::Ge, Value::Number(3), false);
    assert_eq!(
        render(&form.query()),
        "mv>=3 mv<=4",
        "the upper bound is untouched"
    );
}

#[test]
fn every_key_is_drawn_with_some_control() {
    assert_eq!(Control::of(&Key::Color), Control::Colors);
    assert_eq!(Control::of(&Key::Identity), Control::Colors);
    assert_eq!(Control::of(&Key::ManaValue), Control::Number);
    assert_eq!(Control::of(&Key::Power), Control::Number);
    assert_eq!(Control::of(&Key::Is), Control::Flag);
    assert_eq!(Control::of(&Key::Mana), Control::Cost);
    assert_eq!(Control::of(&Key::Type), Control::Text);
    assert_eq!(
        Control::of(&Key::Unknown("frobnicate".to_string())),
        Control::Text,
        "a key nothing knows is still a box a player can read and edit"
    );
}

/// The dialog can say which rows this surface will answer nothing for.
///
/// Not a refusal — the row is drawn and the term is kept, because a player
/// may be building a line to use elsewhere. It is the warning beside the
/// control, and it is the only place the client can say "this will hide
/// everything" before it does.
#[test]
fn a_row_this_surface_cannot_answer_is_named_rather_than_refused() {
    let form = FilterForm::of(&parse("t:creature o:draw m:{G}"));
    assert_eq!(
        form.unanswerable(Surface::POOL),
        Vec::<usize>::new(),
        "a printing answers all three"
    );
    assert_eq!(
        form.unanswerable(Surface::ZONE),
        vec![1, 2],
        "a projected object has neither rules text nor a printed cost"
    );
    // A branch counts as answerable if any of it is: that is `Match`'s own
    // rule for an `or`, and warning about `(t:elf or o:draw)` in a zone would
    // be warning about a line that works.
    let mixed = FilterForm::of(&parse("(t:elf or o:draw)"));
    assert_eq!(mixed.unanswerable(Surface::ZONE), Vec::<usize>::new());
    let neither = FilterForm::of(&parse("(o:draw or m:{G})"));
    assert_eq!(neither.unanswerable(Surface::ZONE), vec![0]);
}

/// A form built from nothing writes a query a player could have typed.
#[test]
fn a_form_built_from_nothing_writes_an_ordinary_line() {
    let mut form = FilterForm::default();
    form.set(
        &Key::Type,
        Op::Colon,
        Value::Word("creature".to_string()),
        false,
    );
    form.set(&Key::Color, Op::Colon, Value::Colors(Colors::W), false);
    form.parts.push(FilterPart::Row {
        negated: true,
        term: Term {
            key: Key::Is,
            op: Op::Colon,
            value: Value::Flag(crate::cardquery::Flag::Stub),
        },
    });
    assert_eq!(render(&form.query()), "t:creature c:w -is:stub");
    assert_eq!(
        parse(&render(&form.query())),
        form.query(),
        "and it reads back as the thing the controls said"
    );
}
