//! The questions answered away from the table - a number, an order, a colour, a creature type, a seat, keep-or-mulligan, yes-or-no - where the answer is a row in a list or a button rather than something a player can point at, which is why a mulligan sits beside a colour here. They share one rule: a value the engine did not offer is inexpressible, so X clamps into the offered range and starts at its minimum, an index past the end is refused, an order is unsubmittable until every offered object is in it exactly once, and every indexed choice reports the row it picked so one renderer row serves all of them. Answering one mode's question in another mode's shape yields `None` rather than an action.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn x_is_clamped_to_the_range_the_engine_offered() {
    let mut i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 50,
    });
    assert_eq!(i.set_number(7), 7);
    // The client cannot express a value outside the offered range, so the
    // usual overflow tricks are simply unavailable to a player.
    assert_eq!(i.set_number(u32::MAX), 50);
    assert_eq!(i.set_number(4_000_000_000), 50);
    assert_eq!(i.confirm(), Some(PlayerAction::ChooseNumber(50)));
}

#[test]
fn x_starts_at_the_minimum() {
    let i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 3,
        max: 9,
    });
    assert_eq!(i.number(), 3);
}

/// An ordering starts complete — every card in its one pile, as offered —
/// and a tap takes a card up while the next puts it down in front of the
/// card tapped. No card is ever missing from the answer, so there is no
/// half-built order to refuse, and holding a card does not stop the answer
/// from being sent as it stands.
#[test]
fn an_ordering_starts_as_offered_and_a_tap_pair_moves_one_card() {
    let mut i = interaction(put_back(vec![obj(1), obj(2), obj(3)]));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::Arrange {
            piles: vec![vec![obj(1), obj(2), obj(3)]]
        }),
        "the order they were offered in is an answer"
    );
    assert_eq!(i.toggle(obj(3)), SelectionOutcome::Added);
    assert!(i.is_selected(obj(3)), "a tapped card is held");
    assert!(i.can_confirm(), "and holding one blocks nothing");
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
    assert!(!i.is_selected(obj(3)), "putting it down lets go of it");
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::Arrange {
            piles: vec![vec![obj(3), obj(1), obj(2)]]
        }),
        "one pile, listed top to bottom"
    );
    assert_eq!(i.arrange_place(obj(3)), Some(1));
    assert_eq!(i.arrange_place(obj(2)), Some(3));
}

/// A keyboard reaches everything a pointer does: the walk stands on a card,
/// the tick takes it up, a nudge moves it, and `Esc` lets go before it
/// undoes anything that was already put down.
#[test]
fn an_ordering_is_played_from_the_keyboard() {
    let mut i = interaction(put_back(vec![obj(1), obj(2), obj(3)]));
    assert_eq!(
        i.aim(),
        Some(Pick::Object(obj(1))),
        "the focus starts on top"
    );
    assert_eq!(i.focus_position(), Some((0, 3)));
    assert_eq!(i.cycle_focus(1), Some(Pick::Object(obj(2))));
    assert_eq!(i.toggle_focused(), SelectionOutcome::Added);
    assert!(i.nudge(Nudge::Later), "one place down");
    assert_eq!(
        i.arrangement().map(|a| a.cards(Row::Pile(0)).to_vec()),
        Some(vec![obj(1), obj(3), obj(2)])
    );
    assert!(i.nudge(Nudge::Earlier) && i.nudge(Nudge::Earlier));
    assert!(!i.nudge(Nudge::Earlier), "already on top");
    assert!(!i.nudge(Nudge::NextRow), "one pile: no row below");
    assert_eq!(i.take_back(), Some(Pick::Object(obj(2))), "Esc lets go");
    assert_eq!(
        i.arrangement().map(|a| a.cards(Row::Pile(0)).to_vec()),
        Some(vec![obj(2), obj(1), obj(3)]),
        "and keeps the move it made"
    );
    assert_eq!(i.take_back(), None, "nothing is held to let go of");
    i.cancel();
    assert_eq!(
        i.arrangement().map(|a| a.cards(Row::Pile(0)).to_vec()),
        Some(vec![obj(1), obj(2), obj(3)]),
        "a cancel with nothing held puts every card back"
    );
}

/// The same question sent again keeps what the player built; a different
/// one starts over. A choice is re-sent whole with every view, and one
/// that threw the arrangement away would undo a player's moves whenever
/// anybody at the table did anything.
#[test]
fn the_same_arrangement_asked_again_keeps_its_moves() {
    let mut i = interaction(put_back(vec![obj(1), obj(2), obj(3)]));
    i.toggle(obj(3));
    i.toggle(obj(1));
    let again = Interaction::new_keeping(put_back(vec![obj(1), obj(2), obj(3)]), me(), Some(&i));
    assert_eq!(
        again.confirm(),
        Some(PlayerAction::Arrange {
            piles: vec![vec![obj(3), obj(1), obj(2)]]
        })
    );
    let other = Interaction::new_keeping(put_back(vec![obj(1), obj(2)]), me(), Some(&i));
    assert_eq!(
        other.confirm(),
        Some(PlayerAction::Arrange {
            piles: vec![vec![obj(1), obj(2)]]
        }),
        "other cards are another question"
    );
}

#[test]
fn ordering_rejects_objects_that_were_not_offered() {
    let mut i = interaction(put_back(vec![obj(1), obj(2)]));
    assert_eq!(i.toggle(obj(42)), SelectionOutcome::Rejected);
}

#[test]
fn a_colour_choice_only_accepts_offered_colours() {
    let mut i = interaction(Pending::ChooseColor {
        player: me(),
        options: vec![ManaColor::White, ManaColor::Blue],
    });
    assert!(!i.can_confirm());
    assert!(!i.choose_index(2), "index beyond the offered options");
    assert!(i.choose_index(1));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseColor(ManaColor::Blue))
    );
}

/// The one pending choice the client could not answer at all. It is
/// worth a test of its own rather than a line in the colour one: the
/// mode was `Idle`, so every accessor said "nothing to do here" and the
/// game simply stopped.
#[test]
fn a_creature_type_choice_is_answerable() {
    let types: Vec<SubtypeId> = (0..350).map(SubtypeId::new).collect();
    let mut i = interaction(Pending::ChooseSubtype {
        player: me(),
        options: types.clone(),
    });
    assert!(!i.can_confirm(), "nothing is picked yet");
    assert_eq!(i.confirm(), None);
    assert!(!i.choose_index(350), "index beyond the offered types");
    assert!(i.choose_index(11));
    assert_eq!(i.chosen_index(), Some(11));
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseSubtype(types[11])),
        "the answer names the type at the picked position"
    );
}

/// Every choice answered by position answers the same way, which is what
/// lets one chooser row in the renderer serve all four.
#[test]
fn every_indexed_choice_reports_the_row_it_picked() {
    let cases = [
        Pending::ChooseColor {
            player: me(),
            options: vec![ManaColor::Blue, ManaColor::Black],
        },
        Pending::ChoosePlayer {
            player: me(),
            options: vec![PlayerId::new(0), PlayerId::new(1)],
        },
        Pending::ChooseSubtype {
            player: me(),
            options: vec![SubtypeId::new(0), SubtypeId::new(1)],
        },
    ];
    for pending in cases {
        let mut i = interaction(pending);
        assert_eq!(i.chosen_index(), None, "nothing is picked to begin with");
        assert!(i.choose_index(1));
        assert_eq!(i.chosen_index(), Some(1));
        assert!(i.confirm().is_some(), "a picked row is submittable");
    }
}

#[test]
fn a_player_choice_only_accepts_offered_seats() {
    let mut i = interaction(Pending::ChoosePlayer {
        player: me(),
        options: vec![PlayerId::new(2), PlayerId::new(3)],
    });
    assert!(!i.choose_index(5));
    assert!(i.choose_index(0));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChoosePlayer(PlayerId::new(2)))
    );
}

#[test]
fn mulligan_and_yes_no_answers_are_mode_gated() {
    let mull = interaction(Pending::Mulligan {
        player: me(),
        taken: 1,
        next_is_free: false,
    });
    assert_eq!(mull.answer_mulligan(true), Some(PlayerAction::MulliganKeep));
    assert_eq!(
        mull.answer_mulligan(false),
        Some(PlayerAction::MulliganTake)
    );
    // A mulligan is not a yes/no question, and answering it as one is not
    // possible.
    assert_eq!(mull.answer_yes_no(true), None);

    let yn = interaction(Pending::YesNo {
        player: me(),
        prompt: YesNoPrompt::Generic,
        source: None,
    });
    assert_eq!(yn.answer_yes_no(true), Some(PlayerAction::YesNo(true)));
    assert_eq!(yn.answer_mulligan(true), None);
}
