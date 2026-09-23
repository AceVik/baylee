//! While a question is standing the sheet belongs to it, and everything that follows from that is here: the tab is pinned when the choice lives in one zone and pinned to nothing when it reaches two, the table is dimmed only where every answer really is inside the sheet, the footer is drawn only on the sheet the answer will be sent from, an ordering numbers each pick in the order it was taken, and the focused row says where the keyboard is standing without thereby claiming to be part of the answer. The counter-cases belong here too, because each is the same predicate answered no: a sheet with no question in front of it darkens nothing, sends nothing and stands on no row.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// W3: a search is about the cards being shown and about nothing else,
/// so the tabs beside them are not part of the question.
#[test]
fn a_question_that_lives_in_one_zone_pins_the_tab_to_it() {
    let shown: Vec<_> = (10..13).map(|s| printed(s, 0, "Forest", 1)).collect();
    let buried: Vec<_> = (20..22).map(|s| printed(s, 0, "Mountain", 1)).collect();
    let view = ViewBuilder::new(2)
        .with_looking_at(shown)
        .with_graveyard(0, buried)
        .build();
    let search = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: (10..13).map(obj).collect(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::SearchLibrary,
        },
        me(),
    );

    let mut b = Browser::new();
    b.follow(&view, Some(&search));
    assert_eq!(b.locked(), Some(BrowseZone::Looking));
    assert_eq!(ticks(&b), vec![BrowseZone::Looking]);
    assert!(
        b.rows(&view, Some(&search), Names::projected())
            .iter()
            .all(|r| r.zone == BrowseZone::Looking),
        "the graveyard has nothing to answer here"
    );

    // The pin is the model's, not the renderer's: a click that reached
    // "every zone" anyway changes nothing.
    b.show(None);
    assert_eq!(ticks(&b), vec![BrowseZone::Looking], "the pin holds");

    // A question that reaches two zones pins nothing — there is no one
    // tab that could answer it.
    let across = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: [obj(10), obj(20)].into(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    b.follow(&view, Some(&across));
    assert_eq!(b.locked(), None);
    assert_eq!(ticks(&b), vec![], "every zone at once");

    // And the pin belongs to the question: answered, the sheet the player
    // opens next is theirs to steer again.
    b.follow(&view, Some(&search));
    assert_eq!(b.locked(), Some(BrowseZone::Looking));
    b.close();
    assert_eq!(b.locked(), None);
    b.open();
    b.show(Some(BrowseZone::Graveyard(me())));
    assert_eq!(ticks(&b), vec![BrowseZone::Graveyard(me())]);
}

/// W2: the dim says "there is nothing else to do", so it is drawn only
/// when that is true — which is a narrower thing than "a question opened
/// this sheet".
#[test]
fn the_table_goes_dark_only_when_every_answer_is_in_the_sheet() {
    let shown: Vec<_> = (10..13).map(|s| printed(s, 0, "Forest", 1)).collect();
    let view = ViewBuilder::new(2)
        .with_looking_at(shown)
        .with_hand(vec![("Ornithopter", 0, 30)])
        .build();
    let mut b = Browser::new();
    assert!(!b.dims_the_table(), "a shut sheet darkens nothing");

    // A search: every card it offers is in the one pile it put on screen.
    let search = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: (10..13).map(obj).collect(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::SearchLibrary,
        },
        me(),
    );
    b.follow(&view, Some(&search));
    assert!(b.dims_the_table(), "nothing outside the sheet is an answer");

    // A question that also offers a card in hand. The sheet still opens —
    // the revealed cards are nowhere else — but the hand is an answer,
    // and a veil over it would be darkening the thing to click.
    let spanning = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: [obj(10), obj(30)].into(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    b.follow(&view, Some(&spanning));
    assert!(b.for_choice(), "a question is still what opened it");
    assert!(
        !b.dims_the_table(),
        "the hand holds an answer and must stay lit"
    );

    // And a pile the player opened to read stands over a live game.
    let mut by_hand = Browser::new();
    by_hand.open_at(BrowseZone::Graveyard(me()));
    assert!(!by_hand.dims_the_table(), "the game goes on underneath");
}

/// One question, one Confirm. The dialog's footer is where the answer is
/// sent from, and the prompt slip reads this same predicate to keep out of
/// its way.
#[test]
fn only_the_sheet_the_question_opened_draws_its_footer() {
    let shown: Vec<_> = (10..13).map(|s| printed(s, 0, "Forest", 1)).collect();
    let view = ViewBuilder::new(2)
        .with_looking_at(shown)
        .with_hand(vec![("Ornithopter", 0, 30)])
        .with_battlefield(0, [printed(40, 0, "Grizzly Bears", 2)])
        .with_graveyard(0, vec![printed(50, 0, "Lightning Bolt", 3)])
        .build();

    let search = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: (10..13).map(obj).collect(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::SearchLibrary,
        },
        me(),
    );
    let mut b = Browser::new();
    assert!(
        !b.answers_here(Some(&search)),
        "a shut sheet answers nothing"
    );
    b.follow(&view, Some(&search));
    assert!(b.answers_here(Some(&search)), "this is the question's home");
    assert!(
        !b.answers_here(None),
        "and a sheet with no question left in it draws no footer either"
    );

    // A choice that spans the sheet and the hand: the table stays lit, and
    // the send still happens here, because there is nowhere else for it.
    let spanning = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: [obj(10), obj(30)].into(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    b.follow(&view, Some(&spanning));
    assert!(!b.dims_the_table(), "the hand is an answer");
    assert!(
        b.answers_here(Some(&spanning)),
        "and this is still the send"
    );

    // The case the footer used to get wrong: a question about the
    // battlefield, and a graveyard the player opened to read while they
    // think about it. Nothing in that pile is an answer.
    let on_the_table = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: [obj(40)].into(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    let mut by_hand = Browser::new();
    by_hand.open_at(BrowseZone::Graveyard(me()));
    by_hand.follow(&view, Some(&on_the_table));
    assert!(by_hand.is_open(), "the pile the player opened stays open");
    assert!(
        !by_hand.answers_here(Some(&on_the_table)),
        "the table is where that question is answered"
    );
}

#[test]
fn an_ordering_opens_the_tray_and_numbers_each_pick() {
    let view = ViewBuilder::new(2)
        .with_looking_at(vec![
            printed(7, 0, "Ponder", 6),
            printed(8, 0, "Brainstorm", 7),
        ])
        .build();
    let mut it = Interaction::new(put_back(vec![obj(7), obj(8)]), me());
    assert!(Browser::wanted(&view, &it), "an ordering always wants it");

    let b = Browser::new();
    assert!(
        b.rows(&view, Some(&it), Names::projected())
            .iter()
            .all(|r| r.place.is_none()),
        "nothing picked yet"
    );
    it.toggle(obj(8));
    it.toggle(obj(7));
    let rows = b.rows(&view, Some(&it), Names::projected());
    let place = |id| rows.iter().find(|r| r.id == id).and_then(|r| r.place);
    assert_eq!(place(obj(8)), Some(1), "picked first, so it goes first");
    assert_eq!(place(obj(7)), Some(2));
}

/// Where the keyboard is standing has to reach the row, or the key that
/// ticks it is ticking something the player cannot pick out of a list.
///
/// One row at a time, and never the chosen one by accident: `focused`
/// and `selected` are two different claims about the same row — the
/// client saying where a press would land, and the answer itself.
#[test]
fn the_row_the_keyboard_stands_on_says_so() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(4, 0, "Llanowar Elves", 3),
                printed(5, 0, "Forest", 4),
            ],
        )
        .build();
    let mut it = Interaction::new(
        baylee_engine::choice::Pending::ChooseCards {
            player: PlayerId::new(0),
            options: vec![ObjectId::new(4, 0), ObjectId::new(5, 0)],
            min: 0,
            max: 2,
            prompt: baylee_engine::choice::ChoicePrompt::Generic,
        },
        PlayerId::new(0),
    );
    let b = Browser::new();
    let focus_of = |it: &Interaction| -> Vec<bool> {
        b.rows(&view, Some(it), Names::projected())
            .iter()
            .map(|row| row.standing.focused)
            .collect()
    };
    assert_eq!(focus_of(&it), vec![true, false], "it starts on the first");
    it.cycle_focus(1);
    assert_eq!(focus_of(&it), vec![false, true], "and the walk moves it");
    // Ticking the second leaves the focus exactly where it was: one row
    // is chosen, the same row is focused, and they are still two flags.
    it.toggle_focused();
    let rows = b.rows(&view, Some(&it), Names::projected());
    assert_eq!(
        rows.iter()
            .map(|r| (r.standing.focused, r.standing.selected))
            .collect::<Vec<_>>(),
        vec![(false, false), (true, true)]
    );
}

/// A panel with no question in front of it stands on nothing.
#[test]
fn a_browse_with_no_question_focuses_no_row() {
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(4, 0, "Forest", 3)])
        .build();
    let rows = Browser::new().rows(&view, None, Names::projected());
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].standing.focused);
}
