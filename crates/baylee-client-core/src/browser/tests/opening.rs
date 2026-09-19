//! What puts the sheet on screen, what takes it away again, and whose doing each of those is: a reveal the seat is shown opens it edge-triggered on the ids and the reveal ending closes it, a search that is answered shuts the sheet it opened, and a panel the player opened by hand outlives every question that arrives after it. The pile tabs beside the mat are the other door in, including the pile that is not a door at all - a library opens nothing, because nobody may look through one (CR 401.2). Only the open flag and its ownership are under test here; which ids a choice offers belongs with the offers, and what a question does to a sheet it is holding belongs with the answering.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A reveal with no question attached opens the sheet by itself.
///
/// This is the job the "Zones" chip used to do and the reason the chip
/// could not simply be deleted: cards in `looking_at` are shown to a seat
/// without anything being asked of them, they are drawn on no other
/// surface in the client, and `follow` only ever runs when a *choice*
/// arrives. Edge-triggered on the ids, so the player can put it away.
/// A library has no tab, so a tap on one has nowhere to go.
///
/// The second of the two readings that enforce CR 401.2 — `ZonePile::
/// is_browsable` is the other — and the one that would silently start
/// working if a `Looking`-shaped variant were ever added for libraries.
#[test]
fn a_library_has_no_tab_to_open() {
    use crate::layout::PileKind;

    let seat = PlayerId::new(0);
    assert_eq!(BrowseZone::of_pile(PileKind::Library, seat), None);
    assert_eq!(
        BrowseZone::of_pile(PileKind::Graveyard, seat),
        Some(BrowseZone::Graveyard(seat))
    );
    assert_eq!(
        BrowseZone::of_pile(PileKind::Exile, seat),
        Some(BrowseZone::Exile(seat))
    );
    // Both command piles are one zone (CR 408.1): a seat with two
    // commanders has two places on the mat and one tab.
    assert_eq!(
        BrowseZone::of_pile(PileKind::Command2, seat),
        BrowseZone::of_pile(PileKind::Command, seat)
    );
}

#[test]
fn cards_shown_to_a_seat_open_the_sheet_by_themselves() {
    let mut b = Browser::new();
    let nothing = ViewBuilder::new(2).build();
    b.saw_reveal(&nothing);
    assert!(!b.is_open(), "an empty reveal is not a reveal");

    let shown = ViewBuilder::new(2)
        .with_looking_at(vec![printed(10, 0, "Ponder", 1)])
        .build();
    b.saw_reveal(&shown);
    assert!(
        b.is_open(),
        "cards being shown open the sheet that draws them"
    );
    assert_eq!(ticks(&b), vec![BrowseZone::Looking]);

    // …and it stays closed once the player closes it, however many views
    // arrive carrying the same cards. A per-frame decision would make the
    // panel impossible to dismiss.
    b.close();
    for _ in 0..5 {
        b.saw_reveal(&shown);
        assert!(!b.is_open(), "the same reveal re-opened it");
    }

    // A reveal that ends and another that begins is two reveals, and the
    // second one opens it again — which a length comparison would miss,
    // because both are one card.
    let other = ViewBuilder::new(2)
        .with_looking_at(vec![printed(11, 0, "Brainstorm", 2)])
        .build();
    b.saw_reveal(&other);
    assert!(b.is_open(), "a different reveal is a new one");

    // But the *same* cards in a different order are the same reveal. A
    // scry is exactly that — every rearrangement comes back as a view —
    // and a sheet the player had closed must not reappear on each one.
    let top = printed(20, 0, "Island", 3);
    let under = printed(21, 0, "Opt", 4);
    let ordered = ViewBuilder::new(2)
        .with_looking_at(vec![top.clone(), under.clone()])
        .build();
    let swapped = ViewBuilder::new(2)
        .with_looking_at(vec![under, top])
        .build();
    b.saw_reveal(&ordered);
    b.close();
    b.saw_reveal(&swapped);
    assert!(!b.is_open(), "reordering the same cards reopened the sheet");
}

/// A search that opened the sheet closes it again when it is answered.
///
/// The fetchland's round trip, which is what the owner asked for: the
/// library goes on screen, a land is picked, and the next question wants
/// nothing from the sheet — so the sheet gets out of the way instead of
/// standing over the board until somebody closes it.
#[test]
fn a_sheet_opened_for_a_search_shuts_when_the_search_is_answered() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .build();
    let search = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: vec![obj(4)],
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    let mut b = Browser::new();
    b.follow(&view, Some(&search));
    assert!(b.is_open(), "the search did not open it");

    // The answer went in; the engine's next question is about the board.
    let after = Interaction::new(
        Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1)],
            player_options: Vec::new(),
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        me(),
    );
    b.follow(&view, Some(&after));
    assert!(!b.is_open(), "the sheet stayed open with nothing to say");
}

/// But a sheet the *player* opened is never closed behind their back.
#[test]
fn a_sheet_opened_by_hand_survives_the_next_question() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .build();
    let mut b = Browser::new();
    b.open_at(BrowseZone::Graveyard(me()));
    let it = Interaction::new(
        Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1)],
            player_options: Vec::new(),
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        me(),
    );
    b.follow(&view, Some(&it));
    assert!(b.is_open(), "the player's own sheet was closed for them");
}

/// A reveal opens the sheet and the reveal ending closes it.
#[test]
fn a_reveal_takes_its_sheet_away_with_it() {
    let shown = ViewBuilder::new(2)
        .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
        .build();
    let done = ViewBuilder::new(2).build();
    let mut b = Browser::new();
    b.saw_reveal(&shown);
    assert!(b.is_open(), "the reveal did not open it");
    b.saw_reveal(&done);
    assert!(!b.is_open(), "the sheet outlived what it was showing");
}

/// A tap that lands on a pile is not a request to abandon the search.
///
/// `open_at` is the only door that writes `tab` without asking the lock,
/// and it also turns a `ForChoice` opening into a by-hand one — so a tap
/// on a graveyard while a library search stood open took the sheet away
/// from the question, `answers_here` went false, and the question's own
/// keys stopped working on a dialog that was still on the screen.
#[test]
fn a_tap_on_a_pile_does_not_take_the_sheet_from_a_question() {
    let view = ViewBuilder::new(2)
        .with_looking_at(vec![printed(4, 0, "Forest", 3)])
        .build();
    let it = Interaction::new(
        baylee_engine::choice::Pending::ChooseCards {
            player: PlayerId::new(0),
            options: vec![ObjectId::new(4, 0)],
            min: 1,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::Generic,
        },
        PlayerId::new(0),
    );
    let mut b = Browser::new();
    b.follow(&view, Some(&it));
    assert!(b.answers_here(Some(&it)), "the sheet holds the question");

    b.open_at(BrowseZone::Graveyard(PlayerId::new(0)));

    assert!(
        b.answers_here(Some(&it)),
        "a tap on a pile took the sheet away from the question"
    );
    assert_eq!(
        ticks(&b),
        vec![BrowseZone::Looking],
        "and it must not have moved the tab either"
    );
}

/// The counter-test: with no question standing, a tap on a pile is
/// exactly what opens that pile, which is the whole job of `open_at`.
#[test]
fn a_tap_on_a_pile_still_opens_that_pile() {
    let mut b = Browser::new();
    b.open_at(BrowseZone::Graveyard(PlayerId::new(0)));
    assert!(b.is_open());
    assert_eq!(ticks(&b), vec![BrowseZone::Graveyard(PlayerId::new(0))]);
}

/// The tray's button is a toggle, and what it toggles keeps everything.
///
/// "Minimised" and "closed" are one state here, which is the whole reason
/// the button exists: what makes the word honest is not a fourth
/// [`Opening`] but the fact that the button is on the ledge either way, so
/// a sheet that is down is one click from being up again with the ticks
/// and the filter the player left in it. This is the test that the click
/// actually returns those and does not hand back a fresh panel.
#[test]
fn the_tray_button_puts_the_sheet_away_and_brings_it_back_as_it_was() {
    let mut b = Browser::new();
    b.open_at(BrowseZone::Graveyard(me()));
    b.set_filter("t:land");
    assert!(b.is_open());

    assert!(b.toggle_by_hand(), "the button refused a sheet it owns");
    assert!(!b.is_open(), "the sheet did not go away");

    assert!(b.toggle_by_hand(), "the button refused to bring it back");
    assert!(b.is_open(), "the sheet did not come back");
    assert_eq!(
        ticks(&b),
        vec![BrowseZone::Graveyard(me())],
        "it came back showing a different zone"
    );
    assert_eq!(
        b.filter().trim(),
        "t:land",
        "it came back with the filter thrown away"
    );
}

/// And it refuses while a question owns the sheet.
///
/// A question opens this panel because one of its answers is somewhere the
/// table cannot show. A tray button that put that away would leave the
/// player holding a question, no way to answer it and no sign of where it
/// went — which is exactly the shape of the fault
/// `a_tap_on_a_pile_does_not_take_the_sheet_from_a_question` records from
/// the other door.
#[test]
fn the_tray_button_does_not_take_a_question_off_the_screen() {
    let view = ViewBuilder::new(2)
        .with_looking_at(vec![printed(4, 0, "Forest", 3)])
        .build();
    let it = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: vec![ObjectId::new(4, 0)],
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    let mut b = Browser::new();
    b.follow(&view, Some(&it));
    assert!(b.answers_here(Some(&it)), "the sheet holds the question");

    assert!(
        !b.may_be_put_away(),
        "the sheet claims it may be put away while a question owns it"
    );
    assert!(!b.toggle_by_hand(), "the button claimed it did something");
    assert!(b.is_open(), "the question was taken off the screen");
    assert!(b.answers_here(Some(&it)), "and its keys stopped working");
}
