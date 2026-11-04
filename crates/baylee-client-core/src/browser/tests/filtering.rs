//! The search box, both halves of it: what a typed string matches - the name drawn on the row and the English name under it, folded so that case, an accent and a keyboard with no umlaut all still find the card - and the field itself, which carries a caret and a selection, takes keys only while it has been handed the keyboard, and strips the control characters out of anything pasted into it. The alphabet is tested here rather than with the sort because the folding is one seam serving both ends of the panel: a list that filed the accent under `z` and a box that could not type it are the same defect twice. `Names` is stood in for by a closure; nothing here reaches a catalog, and the caret arithmetic itself belongs to `textbuf`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn the_filter_narrows_by_name_and_ignores_case() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(4, 0, "Llanowar Elves", 3),
                printed(5, 0, "Forest", 4),
            ],
        )
        .build();
    let mut b = Browser::new();
    b.set_filter("ELV");
    let rows = b.rows(&view, None, Names::projected());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Llanowar Elves");

    b.set_filter("  ");
    assert_eq!(
        b.rows(&view, None, Names::projected()).len(),
        2,
        "blank is not a filter"
    );
}

/// The panel drew one name and searched another.
///
/// A seat reading German sees *Wald* on the row — the renderer has
/// translated the drawn name since the catalog existed — and typing
/// `Wald` into the box under it found nothing, because the filter was
/// asking `object.name`, which the engine keeps in its one language.
/// Both names answer now: the one on the row, and the one the card is
/// known by everywhere outside this client.
#[test]
fn the_filter_answers_the_name_on_the_row_and_the_one_under_it() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(4, 0, "Forest", 0),
                printed(5, 0, "Swamp", 0),
                printed(6, 0, "Llanowar Elves", 3),
            ],
        )
        .build();
    // The catalog, stood in for: this seat's printings are German.
    let german = |object: &baylee_view::PublicObject| match object.name.as_str() {
        "Forest" => Some("Wald".to_string()),
        "Swamp" => Some("Sumpf".to_string()),
        _ => None,
    };
    let names = Names {
        shown: &german,
        ..Names::projected()
    };
    let found = |needle: &str| {
        let mut b = Browser::new();
        b.set_filter(needle);
        b.rows(&view, None, names)
            .into_iter()
            .map(|r| r.name)
            .collect::<Vec<_>>()
    };

    assert_eq!(found("wald"), ["Wald"], "the name the player is looking at");
    assert_eq!(found("forest"), ["Wald"], "the name they learned it under");
    assert_eq!(found("wal"), ["Wald"], "a prefix, which is how one types");
    assert!(found("sumpf") == ["Sumpf"] && found("mountain").is_empty());
    // A card the catalog has no German printing of keeps its own name,
    // and is still found by it — the fallback is a row, not a hole.
    assert_eq!(found("elves"), ["Llanowar Elves"]);
}

/// Both ends of the panel read the same alphabet.
///
/// The seam put German names on the rows and left them being compared as
/// bytes, which files every accented letter above `z`: a graveyard sorted
/// by name put *Ätherfluss* at the bottom, under *Zombie*. The filter had
/// the other half of it — nothing typed on a keyboard without an `ß`
/// could ever find a card printed with one.
#[test]
fn the_panel_alphabetises_and_searches_in_the_readers_own_letters() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(4, 0, "Zombie", 2),
                printed(5, 0, "Aetherflux", 3),
                printed(6, 0, "Brainstorm", 1),
            ],
        )
        .build();
    // The same three cards, as this seat's printings name them.
    let german = |object: &baylee_view::PublicObject| {
        Some(match object.name.as_str() {
            "Aetherflux" => "Ätherfluss".to_string(),
            same => same.to_string(),
        })
    };
    let names = Names {
        shown: &german,
        ..Names::projected()
    };

    let mut b = Browser::new();
    b.sort_by(SortKey::Name);
    let order: Vec<String> = b
        .rows(&view, None, names)
        .into_iter()
        .map(|r| r.name)
        .collect();
    assert_eq!(
        order,
        ["Ätherfluss", "Brainstorm", "Zombie"],
        "the accent belongs at the front, with the A it is one of"
    );

    b.set_filter("atherfluss");
    assert_eq!(
        b.rows(&view, None, names).len(),
        1,
        "a keyboard with no umlaut still finds the card"
    );
    b.set_filter("ss");
    assert_eq!(
        b.rows(&view, None, names).len(),
        1,
        "and so does the ss in it"
    );
}

/// The filter box is a field a player focuses, not a keyboard trap.
///
/// `set_filter` existed from the start and nothing ever called it: the
/// panel could sort and scroll, and the one thing the owner asked for by
/// name — "durchsuchbar" — had no way in. It is typed into now, and the
/// bargain is that it has to be *given* the keyboard: a box that took
/// every keystroke while the panel merely stood open would end playing
/// with the graveyard visible.
#[test]
fn the_filter_box_only_types_while_it_holds_the_keyboard() {
    let mut b = Browser::new();
    assert!(!b.is_typing(), "a fresh panel does not own the keyboard");

    b.start_typing();
    assert!(b.is_open(), "focusing the box opens the panel it lives in");
    assert!(b.is_typing());
    let focused = b.typing_epoch();

    for c in "Elv".chars() {
        b.push_filter(c);
    }
    b.push_filter('\n');
    assert_eq!(b.filter(), "Elv", "a control character reached the text");
    assert!(b.pop_filter());
    assert_eq!(b.filter(), "El");

    // The same rule on the path that needs it more. A keystroke is one
    // character a player meant; `set_filter` is a whole value arriving
    // from autofill or a paste, which is where a newline actually comes
    // from — and a filter holding one matches nothing at all.
    b.set_filter("Ll\tanowar\n");
    assert_eq!(
        b.filter(),
        "Llanowar",
        "a pasted value kept its control codes"
    );

    // Focusing again while already focused is not a new focus: a platform
    // input pointed at the box on every frame would fight the player for
    // the caret.
    b.start_typing();
    assert_eq!(b.typing_epoch(), focused);

    // Emptying the box, on the other hand, *is* one — the platform's own
    // field is still holding the old letters until something points it at
    // the new value.
    b.clear_filter();
    assert_eq!(b.filter(), "");
    assert!(b.typing_epoch() > focused, "the field was not re-seeded");
    b.set_filter("El");

    b.stop_typing();
    assert!(!b.is_typing());
    assert_eq!(
        b.filter(),
        "El",
        "letting go of the box threw the text away"
    );

    // And closing the panel lets go: the keyboard belongs to the game
    // again the moment the panel is not on screen.
    b.start_typing();
    b.close();
    assert!(!b.is_typing());
}

/// Typing narrows the rows, which is the whole point of the box.
#[test]
fn what_is_typed_is_what_is_listed() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(1, 0, "Elvish Mystic", 1),
                printed(2, 0, "Mountain", 2),
            ],
        )
        .build();
    let mut b = Browser::new();
    b.open();
    assert_eq!(b.rows(&view, None, Names::projected()).len(), 2);
    b.start_typing();
    for c in "mou".chars() {
        b.push_filter(c);
    }
    let rows = b.rows(&view, None, Names::projected());
    assert_eq!(rows.len(), 1, "the filter did not reach the rows");
    assert_eq!(rows[0].name, "Mountain");
}

/// The search box is a field, not a string with letters pushed onto it.
///
/// The owner asked for it by naming the boxes that already work: *"die
/// Input felder überall, auch das Suchfeld im Zonen-Dialog soll
/// vollständig funktionieren wie ein normales Input Feld aus dem Web.
/// (Sowie die im Login Formullar, die funktionieren top.)"* Everything
/// below is something that box could not do — the caret could only ever
/// be at the end, because that is where `push`ing puts a character.
#[test]
fn the_search_box_has_a_caret_a_selection_and_the_keys_that_move_them() {
    use crate::textbuf::{Dir, Step};
    let mut b = Browser::new();
    b.start_typing();
    b.type_text("Llanowar Elves");

    // Home, then two words to the right, then type in the middle of it.
    b.move_filter_caret(Step::Line, Dir::Left, false);
    assert_eq!(b.filter_field().cursor(), 0);
    b.move_filter_caret(Step::Word, Dir::Right, false);
    assert_eq!(
        b.filter_field().cursor(),
        "Llanowar".len(),
        "a word is a word and not eight presses of the right arrow"
    );
    b.push_filter('!');
    assert_eq!(b.filter(), "Llanowar! Elves");

    // Shift extends a selection, and typing over it replaces it.
    b.move_filter_caret(Step::Line, Dir::Right, false);
    b.move_filter_caret(Step::Word, Dir::Left, true);
    assert_eq!(
        b.filter_field().selection().map(|s| &b.filter()[s]),
        Some("Elves"),
        "shift+⌥← selects the word behind the caret"
    );
    b.type_text("Mystic");
    assert_eq!(b.filter(), "Llanowar! Mystic");

    // Delete rubs out forwards, Backspace backwards, and either one takes
    // the selection whole when there is one.
    b.move_filter_caret(Step::Line, Dir::Left, false);
    b.delete_forward();
    assert_eq!(b.filter(), "lanowar! Mystic");
    b.select_all_filter();
    assert!(b.pop_filter(), "there was a selection to rub out");
    assert_eq!(b.filter(), "", "select-all and one press empties the box");
    assert!(!b.pop_filter(), "and there is nothing left to rub out");

    // A platform that owns its own typing hands the caret over with the
    // value, which is what an `<input>`'s arrow keys and its paste do.
    b.set_filter_state("Forest", 3, Some(6));
    assert_eq!(b.filter(), "Forest");
    assert_eq!(b.filter_field().cursor(), 3);
    assert_eq!(
        b.filter_field().selection().map(|s| &b.filter()[s]),
        Some("est")
    );
    // Unless a control character had to be dropped, which would move every
    // offset after it: then the caret goes to the end rather than
    // somewhere the platform did not mean.
    b.set_filter_state("For\nest", 3, Some(6));
    assert_eq!(b.filter(), "Forest");
    assert_eq!(b.filter_field().cursor(), 6);
    assert_eq!(b.filter_field().selection(), None);
}

/// The zone box reads the same language the deckbuilder's does.
///
/// The owner asked for one search across the client — *"Suche im
/// Deckbuilder, Zonen Dialog, etc. … der Skryfall vergleihbaren
/// Textfilter"* — so this box is not a substring match with a query
/// language bolted on beside it: it *is* the query language, and a bare
/// word is the same `Key::Loose` term it is in the pool.
#[test]
fn a_zone_search_reads_the_same_language_the_deckbuilder_does() {
    use baylee_core::color::{Color, ColorSet};

    let mut elves = printed(4, 0, "Llanowar Elves", 3);
    elves.mana_value = 1;
    elves.colors = ColorSet::of(Color::Green);
    let mut wrath = printed(5, 0, "Wrath of God", 4);
    wrath.types = TypeSet::SORCERY;
    wrath.power = None;
    wrath.toughness = None;
    wrath.mana_value = 4;
    wrath.colors = ColorSet::of(Color::White);
    let mut forest = printed(6, 0, "Forest", 5);
    forest.types = TypeSet::LAND;
    forest.supertypes = baylee_core::types::SupertypeSet::BASIC;
    forest.power = None;
    forest.toughness = None;

    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![elves, wrath, forest])
        .build();
    let found = |needle: &str| {
        let mut b = Browser::new();
        b.set_filter(needle);
        b.rows(&view, None, Names::projected())
            .into_iter()
            .map(|r| r.name)
            .collect::<Vec<_>>()
    };

    assert_eq!(found("t:creature"), ["Llanowar Elves"]);
    assert_eq!(found("t:sorcery"), ["Wrath of God"]);
    assert_eq!(found("c:w"), ["Wrath of God"], "the projected colours");
    assert_eq!(found("mv>=4"), ["Wrath of God"]);
    assert_eq!(found("pow>=2"), ["Llanowar Elves"], "the projected power");
    assert_eq!(found("is:basic"), ["Forest"], "a projected supertype");
    assert_eq!(
        found("-t:creature"),
        ["Wrath of God", "Forest"],
        "a leading minus is a negation and not a letter to search for"
    );
    assert_eq!(
        found("t:creature or t:land"),
        ["Llanowar Elves", "Forest"],
        "and an or is an or"
    );
}

/// A term this surface cannot answer hides every row, both ways round.
///
/// The view carries characteristics and never the card's prose, so `o:`
/// has nothing to read. Answering it as *false* would be a lie a player
/// could not see — `-o:draw` would then list the whole graveyard as cards
/// that do not draw, which nothing here knows.
#[test]
fn a_zone_search_refuses_a_question_the_view_cannot_answer() {
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .build();
    let found = |needle: &str| {
        let mut b = Browser::new();
        b.set_filter(needle);
        b.rows(&view, None, Names::projected()).len()
    };
    assert_eq!(found("o:draw"), 0, "there is no rules text to read");
    assert_eq!(found("-o:draw"), 0, "and negating it does not create one");
    assert_eq!(found("m:{G}"), 0, "nor is there a printed mana cost");
    assert_eq!(found("id:g"), 0, "nor a colour identity");
    assert_eq!(found("elves"), 1, "what it does carry still answers");
}

/// The gear opens a builder on what is in the box, and looking at it changes
/// nothing.
///
/// The sharp half of the whole feature. `Key::render` picks one spelling out
/// of the several Scryfall accepts, so a builder that wrote its own reading
/// back on opening would turn `color:red cmc:3` into `c:r mv:3` — the same
/// search, and a box rewritten by nothing but being looked at.
#[test]
fn opening_the_builder_leaves_the_typed_string_exactly_as_it_was() {
    let mut b = Browser::new();
    b.set_filter("color:red cmc:3");
    b.toggle_builder();
    assert_eq!(b.builder().expect("it is open").parts().len(), 2);
    assert_eq!(b.filter(), "color:red cmc:3", "and the box is untouched");
    b.toggle_builder();
    assert!(b.builder().is_none());
    assert_eq!(b.filter(), "color:red cmc:3");
}

/// A button in the builder writes the box, and the list under it answers.
#[test]
fn a_button_in_the_builder_writes_the_box_and_the_list_follows() {
    let mut forest = printed(5, 0, "Forest", 4);
    forest.types = TypeSet::LAND;
    forest.power = None;
    forest.toughness = None;
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3), forest])
        .build();
    let mut b = Browser::new();
    b.toggle_builder();
    // Add a type row, type into it, and the box says what was built.
    let at = crate::filterdialog::OFFERED
        .iter()
        .position(|key| *key == crate::cardquery::Key::Type)
        .expect("the menu offers a type");
    b.filter_act(crate::filterdialog::Act::Add(at));
    b.filter_act(crate::filterdialog::Act::Edit(0));
    b.in_builder(|panel| panel.type_text("land"));
    assert_eq!(b.filter(), "t:land");
    assert_eq!(
        b.rows(&view, None, Names::projected())
            .into_iter()
            .map(|row| row.name)
            .collect::<Vec<_>>(),
        ["Forest"],
        "and the rows are the ones the built string asks for"
    );
}

/// The box has no caret while the builder is open, and gets it back by being
/// tapped.
#[test]
fn the_box_and_the_builder_are_never_both_holding_the_caret() {
    let mut b = Browser::new();
    b.start_typing();
    assert!(b.is_typing());
    b.toggle_builder();
    assert!(!b.is_typing(), "the gear takes the caret out of the box");
    b.close_builder();
    b.start_typing();
    assert!(b.is_typing(), "and a tap in the box takes it back");
    assert!(b.builder().is_none());
}

/// A table saying card 7 is rare in `m21` and legal in Modern, and 8 only
/// common in `lea` and banned there.
fn pool_facts() -> crate::cardquery::PoolFacts {
    use baylee_cardtext::{CardLine, PoolFactsBody, PoolSet};
    let set = |code: &str| PoolSet {
        code: code.to_string(),
        name: code.to_string(),
    };
    let line = |index, word: &str, set: &str, rarity: &str| CardLine {
        index,
        legalities: [("modern".to_string(), word.to_string())].into(),
        printed: vec![(set.to_string(), rarity.to_string())],
    };
    PoolFactsBody::assemble(
        vec![set("m21"), set("lea")],
        vec![
            line(7, "legal", "m21", "rare"),
            line(8, "banned", "lea", "common"),
        ],
    )
    .into()
}

/// The rows a line in the box leaves, by name.
fn left_by(
    view: &PlayerView,
    line: &str,
    facts: Option<&crate::cardquery::PoolFacts>,
) -> Vec<String> {
    let mut b = Browser::new();
    b.set_filter(line);
    let names = Names {
        facts,
        ..Names::projected()
    };
    b.rows(view, None, names)
        .into_iter()
        .map(|row| row.name)
        .collect()
}

/// These are facts about the card the view already names, so they narrow a
/// zone as they narrow the pool.
#[test]
fn a_zone_answers_rarity_set_and_format_off_the_card_a_row_names() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![printed(4, 0, "Elves", 7), printed(5, 0, "Forest", 8)],
        )
        .build();
    let facts = pool_facts();
    let table = Some(&facts);
    assert_eq!(left_by(&view, "r:rare", table), ["Elves"]);
    assert_eq!(left_by(&view, "s:lea", table), ["Forest"]);
    assert_eq!(left_by(&view, "f:modern", table), ["Elves"]);
    assert_eq!(left_by(&view, "banned:modern", table), ["Forest"]);
    assert_eq!(left_by(&view, "-f:modern", table), ["Forest"]);
}

/// No table, nothing shown, and the negation is no way round it: this is the
/// surface saying it cannot answer, not every card being not rare.
#[test]
fn without_the_table_a_zone_shows_nothing_for_these_keys() {
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(4, 0, "Elves", 7)])
        .build();
    for line in ["r:rare", "-r:rare", "s:m21", "f:modern", "-banned:modern"] {
        assert!(left_by(&view, line, None).is_empty(), "`{line}`");
    }
}

/// The hidden-information rule, on these keys: a row the view names no card
/// for answers nothing, a term and its negation alike. A face-down card and a
/// token read the same here, so a filter cannot tell them apart or learn
/// anything about the one that is a card.
#[test]
fn a_row_the_view_names_no_card_for_answers_no_rarity_set_or_format() {
    let mut face_down = printed(4, 1, "Face-down", 7);
    face_down.card = None;
    face_down.rules = None;
    let token = crate::test_support::token(5, 1, "Soldier", 1, 1);
    let view = ViewBuilder::new(2)
        .with_graveyard(1, vec![face_down, token])
        .build();
    let facts = pool_facts();
    for line in [
        "r:rare",
        "-r:rare",
        "r>=common",
        "s:m21",
        "-s:m21",
        "f:modern",
        "-f:modern",
        "banned:modern",
        "-banned:modern",
    ] {
        assert!(left_by(&view, line, Some(&facts)).is_empty(), "`{line}`");
    }
    assert_eq!(
        left_by(&view, "t:creature", Some(&facts)).len(),
        2,
        "what the rows do say is still asked"
    );
}

#[test]
fn a_zone_with_a_table_says_it_can_answer() {
    let facts = pool_facts();
    let names = Names {
        facts: Some(&facts),
        ..Names::projected()
    };
    assert!(names.surface().answers(&crate::cardquery::Key::Rarity));
    assert!(
        !Names::projected()
            .surface()
            .answers(&crate::cardquery::Key::Rarity)
    );
}
