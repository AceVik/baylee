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
    let names = Names { shown: &german };
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
    let names = Names { shown: &german };

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
