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

// ------------------------------------------- the builder as a mode of a box

use super::{Adding, ColorRule, FilterPanel};
use crate::cardquery::Flag;

/// Opening the builder writes nothing back into the box.
///
/// The sharpest edge in the module. The property that is proved is about
/// *queries* — `FilterForm::of(q).query() == q` — and a box holds a
/// **string**. `Key::render` picks one spelling out of the several a player
/// may have typed, so `color:red` is written back as `c:r` — both halves in
/// another spelling: equal as a query, and a box that rewrote itself because
/// somebody looked at it.
#[test]
fn opening_the_builder_leaves_the_typed_spelling_alone() {
    let panel = FilterPanel::open(&parse("color:red cmc:3"));
    assert_eq!(
        panel.written(),
        None,
        "nothing was changed, so nothing is written"
    );
    assert_eq!(
        panel.parts().len(),
        2,
        "and both rows are there to be drawn"
    );
}

#[test]
fn the_first_change_is_what_writes_the_box() {
    let mut panel = FilterPanel::open(&parse("color:red cmc:3"));
    panel.negate(0, true);
    let written = panel.written().expect("a row was changed");
    assert_eq!(
        render(&written),
        "-c:r mv:3",
        "and now the whole line is written in the language's own spelling"
    );
}

#[test]
fn a_row_the_builder_cannot_draw_survives_being_edited_around() {
    let mut panel = FilterPanel::open(&parse("t:creature (c:r or c:g) mv:3"));
    panel.set_value(2, Value::Number(5));
    assert_eq!(
        render(&panel.written().unwrap()),
        "t:creature (c:r or c:g) mv:5",
        "the branch kept its place and its brackets"
    );
}

/// The colon means the opposite thing on the two colour keys.
#[test]
fn a_colour_reading_is_a_sentence_and_not_an_operator() {
    assert_eq!(
        ColorRule::of(&Key::Color, Op::Colon),
        Some(ColorRule::AtLeast)
    );
    assert_eq!(
        ColorRule::of(&Key::Identity, Op::Colon),
        Some(ColorRule::AtMost)
    );
    assert_eq!(ColorRule::of(&Key::Color, Op::Ge), Some(ColorRule::AtLeast));
    assert_eq!(
        ColorRule::of(&Key::Color, Op::Ne),
        None,
        "an operator with no reading lights nothing rather than the nearest one"
    );
    // And writing a reading back picks the spelling that key already uses.
    assert_eq!(ColorRule::AtLeast.op(&Key::Color), Op::Colon);
    assert_eq!(ColorRule::AtLeast.op(&Key::Identity), Op::Ge);
    assert_eq!(ColorRule::AtMost.op(&Key::Identity), Op::Colon);
    assert_eq!(ColorRule::AtMost.op(&Key::Color), Op::Le);
}

#[test]
fn touching_a_reading_a_term_already_agrees_with_writes_it_back_unchanged() {
    let mut panel = FilterPanel::open(&parse("c:rg"));
    panel.set_rule(0, ColorRule::AtLeast);
    assert_eq!(
        render(&panel.written().unwrap()),
        "c:rg",
        "the colon already said at-least, so no longer synonym is written"
    );
}

#[test]
fn the_colour_pips_add_and_take_away_and_colourless_stands_alone() {
    let mut panel = FilterPanel::open(&parse("c:r"));
    panel.toggle_color(0, 'g');
    assert_eq!(render(&panel.written().unwrap()), "c:rg");
    panel.toggle_color(0, 'r');
    assert_eq!(render(&panel.written().unwrap()), "c:g");
    panel.toggle_color(0, 'c');
    assert_eq!(
        render(&panel.written().unwrap()),
        "c:c",
        "a card is colourless or it is not — `c:gc` is not a question"
    );
}

/// A fresh row is a term that can be written, not a blank waiting for one.
#[test]
fn every_kind_of_row_opens_on_something_writable() {
    for key in super::OFFERED {
        let mut panel = FilterPanel::default();
        panel.add(key);
        let query = panel.written().expect("adding a row is a change");
        let written = render(&query);
        assert_eq!(
            parse(&written),
            query,
            "a fresh {key:?} row was written as `{written}` and read back as \
             something else"
        );
        assert_eq!(
            FilterPanel::open(&parse(&written)).parts().len(),
            1,
            "and opening the box again still finds the row: `{written}`"
        );
    }
}

/// The spellings a fresh row of each kind writes.
///
/// Beside the loop above rather than inside it: that one asserts the
/// *property* over every key the menu offers and grows with the menu, and
/// this one is what a reader wants when they ask what the box says after one
/// tap. Four keys and not thirteen, because what differs is the control.
#[test]
fn a_fresh_row_of_each_kind_writes_what_it_is() {
    for (key, written) in [
        (Key::Type, "t:\"\""),
        (Key::Color, "c:c"),
        (Key::ManaValue, "mv=0"),
        (Key::Is, "is:playable"),
        (Key::Mana, "m:\"\""),
    ] {
        let mut panel = FilterPanel::default();
        panel.add(&key);
        assert_eq!(
            render(&panel.written().expect("adding a row is a change")),
            written,
            "a fresh {key:?} row"
        );
    }
}

/// A cost is built symbol by symbol, and the row outlives its symbols.
///
/// This was the one key the menu treated specially, on a reading of
/// `Value::Cost(vec![])` that was wrong three times over: it renders as
/// `m:""`, it reads back as the same empty cost, and the evaluator answers
/// `Yes` — a cost holding no named symbol is every cost. So there is nothing
/// to protect a player from, and a row deleting itself under the finger that
/// was backspacing in it is worse than one that sits there empty.
#[test]
fn a_cost_row_is_built_symbol_by_symbol_and_goes_only_by_its_own_cross() {
    let mut panel = FilterPanel::default();
    panel.add(&Key::Mana);
    assert_eq!(panel.parts().len(), 1, "the row is there to be typed into");
    assert_eq!(render(&panel.written().unwrap()), "m:\"\"");

    panel.push_symbol(0, "W");
    panel.push_symbol(0, "W");
    panel.push_symbol(0, "2");
    assert_eq!(render(&panel.written().unwrap()), "m:{W}{W}{2}");
    assert_eq!(
        parse(&render(&panel.written().unwrap())),
        panel.written().unwrap()
    );

    panel.pop_symbol(0);
    panel.pop_symbol(0);
    panel.pop_symbol(0);
    assert_eq!(render(&panel.written().unwrap()), "m:\"\"");
    panel.pop_symbol(0);
    assert_eq!(
        panel.parts().len(),
        1,
        "backspacing past the first symbol does nothing — the \u{2715} is how a row goes"
    );
    panel.remove(0);
    assert!(panel.parts().is_empty());
}

#[test]
fn adding_a_condition_closes_the_menu_it_was_added_from() {
    let mut panel = FilterPanel::default();
    panel.add_step(Adding::Kinds);
    assert_eq!(panel.adding(), Adding::Kinds);
    panel.add_step(Adding::Keys(Control::Number));
    assert_eq!(panel.adding(), Adding::Keys(Control::Number));
    panel.add(&Key::Power);
    assert_eq!(
        panel.adding(),
        Adding::Closed,
        "a player who has just added a condition is looking at the condition"
    );
}

/// Removing is how a row's key changes, and how a flag says "don't ask".
#[test]
fn a_row_that_would_stand_for_no_term_is_removed_instead() {
    let mut panel = FilterPanel::open(&parse("t:creature is:stub mv:3"));
    panel.remove(1);
    assert_eq!(render(&panel.written().unwrap()), "t:creature mv:3");
    // Out of range changes nothing and says nothing happened.
    let before = panel.written();
    panel.remove(9);
    assert_eq!(panel.written(), before);
}

#[test]
fn clearing_empties_the_box_rather_than_leaving_brackets() {
    let mut panel = FilterPanel::open(&parse("t:creature (c:r or c:g)"));
    panel.clear();
    assert!(panel.written().unwrap().is_anything());
    assert_eq!(render(&panel.written().unwrap()), "");
    assert!(panel.parts().is_empty());
}

#[test]
fn the_builder_names_the_rows_its_surface_cannot_answer() {
    let panel = FilterPanel::open(&parse("t:creature o:draw"));
    assert_eq!(panel.unanswerable(Surface::ZONE), vec![1]);
    assert!(panel.unanswerable(Surface::POOL).is_empty());
    let _ = Flag::Playable;
}

/// Every button is an [`Act`], and an `Act` is what a test presses.
///
/// The point of the vocabulary: a panel driven through the same list the
/// buttons carry cannot pass while a button is unwired to a method that
/// works. `Interaction::activate` shipped written and unreachable once;
/// a test that called the method could not have told.
#[test]
fn a_whole_filter_can_be_built_by_pressing_buttons() {
    use super::Act;

    let mut panel = FilterPanel::default();
    // "Add a condition" → Number → Mana value.
    panel.act(Act::AddStep(Adding::Kinds));
    panel.act(Act::AddStep(Adding::Keys(Control::Number)));
    let mv = super::OFFERED
        .iter()
        .position(|key| *key == Key::ManaValue)
        .expect("the mana value is offered");
    panel.act(Act::Add(mv));
    panel.act(Act::Bump(0, 3));
    assert_eq!(render(&panel.written().unwrap()), "mv=3");

    // The stepper stops at nought rather than writing a term no card answers.
    panel.act(Act::Bump(0, -9));
    assert_eq!(render(&panel.written().unwrap()), "mv=0");
    panel.act(Act::SetOp(0, Op::Le));
    panel.act(Act::SetNumber(0, 3));
    assert_eq!(render(&panel.written().unwrap()), "mv<=3");

    // A colour row, negated.
    let colour = super::OFFERED
        .iter()
        .position(|key| *key == Key::Color)
        .expect("colour is offered");
    panel.act(Act::Add(colour));
    panel.act(Act::Colour(1, 'w'));
    panel.act(Act::Negate(1, true));
    assert_eq!(render(&panel.written().unwrap()), "mv<=3 -c:w");

    // A cost, filled from the keyboard beside it.
    panel.act(Act::AddStep(Adding::Kinds));
    let mana = super::OFFERED
        .iter()
        .position(|key| *key == Key::Mana)
        .expect("a cost is offered");
    panel.act(Act::Add(mana));
    assert_eq!(panel.parts().len(), 3, "an ordinary row like any other");
    panel.act(Act::PushSymbol(2, 0));
    panel.act(Act::PushSymbol(2, 0));
    assert_eq!(render(&panel.written().unwrap()), "mv<=3 -c:w m:{W}{W}");

    // And the whole line reads back as itself.
    let written = panel.written().unwrap();
    assert_eq!(parse(&render(&written)), written);

    panel.act(Act::Clear);
    assert_eq!(render(&panel.written().unwrap()), "");
}

/// An index that names nothing does nothing, and says so by changing nothing.
///
/// A button drawn a frame ago may name a row a later frame no longer has —
/// a double tap on ✕ is the ordinary way to produce one — so every one of
/// these is a no-op rather than a panic.
#[test]
fn an_act_naming_a_row_that_is_gone_is_quiet() {
    use super::Act;

    let mut panel = FilterPanel::open(&parse("t:creature"));
    let before = render(&FilterForm::of(&parse("t:creature")).query());
    for act in [
        Act::Negate(7, true),
        Act::Remove(7),
        Act::SetOp(7, Op::Eq),
        Act::SetRule(7, ColorRule::AtMost),
        Act::Colour(7, 'w'),
        Act::SetNumber(7, 3),
        Act::Bump(7, 1),
        Act::Parity(7, Some(true)),
        Act::SetFlag(7, 99),
        Act::Add(99),
        Act::Edit(7),
        Act::PushSymbol(7, 0),
        Act::PopSymbol(7),
    ] {
        panel.act(act);
    }
    assert_eq!(panel.parts().len(), 1, "{before} is still the only row");
    assert_eq!(render(&panel.form.query()), before);
}

/// Typing into a row writes the value a player would have typed into the box.
///
/// The same reader, [`value_of`](crate::cardquery::value_of), and not a
/// second one beside it: a row that arrived at a different value from the
/// same letters would be the silent rewrite at the width of one row.
#[test]
fn typing_into_a_row_reads_the_letters_the_language_reads() {
    let mut panel = FilterPanel::default();
    panel.add(&Key::Type);
    panel.edit(0);
    panel.type_text("goblin");
    assert_eq!(render(&panel.written().unwrap()), "t:goblin");

    // A number row is not only a stepper: `mv:12` is two taps of a keyboard
    // and twelve of an arrow, and `even` is on no stepper at all.
    panel.add(&Key::ManaValue);
    panel.edit(1);
    panel.type_text("12");
    assert_eq!(render(&panel.written().unwrap()), "t:goblin mv=12");
    panel.select_all_typed();
    panel.type_text("even");
    assert_eq!(render(&panel.written().unwrap()), "t:goblin mv=even");

    // And a cost row reaches what the six-symbol keyboard cannot spell.
    panel.add(&Key::Mana);
    panel.edit(2);
    panel.type_text("{2}{W/U}{R/P}");
    assert_eq!(
        render(&panel.written().unwrap()),
        "t:goblin mv=even m:{2}{W/U}{R/P}"
    );
}

/// A control tapped while its row is being typed into wins, and the caret
/// starts again from what it wrote.
///
/// Without this the box under the caret still holds the text from before the
/// tap, and the next letter writes it back over the pip — a control and a box
/// editing one value, which is this module's own failure mode at the width of
/// a row.
#[test]
fn a_pip_tapped_under_the_caret_is_what_the_next_letter_starts_from() {
    let mut panel = FilterPanel::default();
    panel.add(&Key::Color);
    panel.edit(0);
    panel.type_text("r");
    assert_eq!(render(&panel.written().unwrap()), "c:r");
    panel.toggle_color(0, 'g');
    assert_eq!(render(&panel.written().unwrap()), "c:rg");
    assert_eq!(
        panel.typing().expect("still typing").field().text(),
        "rg",
        "the box under the caret says what the pips say"
    );
    panel.type_text("w");
    assert_eq!(
        render(&panel.written().unwrap()),
        "c:wrg",
        "so the next letter adds to the pips rather than undoing them — and \
         the written form is WUBRG, which is the order a cost is printed in"
    );
}

/// The caret follows the row it is in, and goes when the row does.
#[test]
fn a_row_removed_under_the_caret_takes_the_caret_with_it() {
    let mut panel = FilterPanel::open(&parse("t:creature mv:3 c:r"));
    panel.edit(2);
    panel.remove(0);
    assert_eq!(
        panel.typing().expect("the caret is still somewhere").row(),
        1,
        "the rows below moved up, and the caret moved with its own"
    );
    panel.type_text("g");
    assert_eq!(
        render(&panel.written().unwrap()),
        "mv:3 c:g",
        "and it is still typing into the colour row and not into the number"
    );
    panel.remove(1);
    assert!(panel.typing().is_none(), "the row it was in is gone");
    panel.type_text("x");
    assert_eq!(
        render(&panel.written().unwrap()),
        "mv:3",
        "and a key pressed after that goes nowhere at all"
    );
}

/// Opening a row for typing selects what is in it.
///
/// A fresh row opens empty and an old one opens on its own spelling, and in
/// both cases the first letter typed should be the value — not appended to
/// what was already there. It is the same thing a search box does when it
/// takes focus.
#[test]
fn taking_a_row_selects_what_it_already_says() {
    let mut panel = FilterPanel::open(&parse("t:creature"));
    panel.edit(0);
    assert_eq!(panel.typing().unwrap().field().text(), "creature");
    panel.type_text("land");
    assert_eq!(render(&panel.written().unwrap()), "t:land");
}

/// A row is drawn by what it **holds**, not by what its key usually holds.
///
/// Found by photographing the dialog: `c>=2` stood there as five unlit pips
/// and `is:permanent` as three unlit choices, each a row saying nothing about
/// its own value — and one press of any of those buttons would have written
/// over it. One key spells more than one kind of value, and the control has
/// to follow the value.
#[test]
fn a_control_follows_the_value_and_not_only_the_key() {
    for (line, want) in [
        ("c:rg", Control::Colors),
        // A count of colours is a comparison and a number, which is exactly
        // what the number control draws.
        ("c>=2", Control::Number),
        // And what neither control can draw is drawn as what was written:
        // `m` is a count wearing a letter, `esper` is a nickname this
        // language refuses, `permanent` is not a flag it knows.
        ("c:m", Control::Text),
        ("c:esper", Control::Text),
        ("is:token", Control::Flag),
        ("is:permanent", Control::Text),
        ("mv:3", Control::Number),
        ("mv:even", Control::Number),
        ("mv:soon", Control::Text),
        ("m:{W}{U}", Control::Cost),
    ] {
        let panel = FilterPanel::open(&parse(line));
        assert_eq!(
            panel.control(0),
            Some(want),
            "`{line}` is drawn with the control its value needs"
        );
    }
    assert_eq!(
        FilterPanel::open(&parse("c:r")).control(7),
        None,
        "and a row that is not there is drawn with nothing"
    );
}

/// The control does not change under the caret.
///
/// Typing goes through `value_of` on every keystroke, so a value's *kind*
/// changes mid-word: `toke` is a word and `token` is a flag. A row that swapped
/// its control on the `n` would take the box out from under the letters while
/// the caret still named that row, and every further keystroke would go into
/// a field nothing drew.
#[test]
fn a_row_being_typed_into_stays_a_text_box() {
    let mut panel = FilterPanel::open(&parse("is:permanent"));
    panel.edit(0);
    for letter in "token".chars() {
        panel.type_text(&letter.to_string());
        assert_eq!(
            panel.control(0),
            Some(Control::Text),
            "the box stays a box while it is being typed into"
        );
    }
    panel.stop_typing();
    assert_eq!(
        panel.control(0),
        Some(Control::Flag),
        "and the flag it now holds is drawn as a flag once the caret is given back"
    );
    assert_eq!(render(&panel.written().unwrap()), "is:token");
}

/// The stepper steps whatever number the row is holding.
///
/// `Value::Number` and `Value::ColorCount` are the same number on the page
/// and the stepper read only the first of them, so `+` on a `c>=2` row read
/// nought and wrote `c>=1` — a row that answered a button by forgetting what
/// it said.
#[test]
fn the_stepper_counts_colours_as_readily_as_mana() {
    let mut panel = FilterPanel::open(&parse("c>=2"));
    panel.act(super::Act::Bump(0, 1));
    assert_eq!(render(&panel.written().unwrap()), "c>=3");
    panel.act(super::Act::Bump(0, -1));
    panel.act(super::Act::Bump(0, -1));
    assert_eq!(render(&panel.written().unwrap()), "c>=1");
    // And the value it wrote is the one the same string parses to, which is
    // what keeps the form and the box saying the same thing.
    assert_eq!(panel.written().unwrap(), parse("c>=1"));
}

/// A number written into a row is the kind of number its key reads.
///
/// `SetNumber` wrote a `Value::Number` whatever the key was. Under `c` that
/// renders the same string and parses back as a `Value::ColorCount`, so the
/// box was right and the form was holding something the box could never have
/// produced — the one state this module exists to prevent.
#[test]
fn a_number_written_into_a_colour_row_is_a_count_of_colours() {
    let mut panel = FilterPanel::open(&parse("c>=2"));
    panel.act(super::Act::SetNumber(0, 4));
    assert_eq!(panel.written().unwrap(), parse("c>=4"));
    let mut panel = FilterPanel::open(&parse("mv:3"));
    panel.act(super::Act::SetNumber(0, 4));
    assert_eq!(panel.written().unwrap(), parse("mv:4"));
}
