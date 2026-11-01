//! The questions answered away from the table - a number, an order, a colour, a creature type, a seat, keep-or-mulligan, yes-or-no - where the answer is a row in a list or a button rather than something a player can point at, which is why a mulligan sits beside a colour here. They share one rule: a value the engine did not offer is inexpressible, so X clamps into the offered range and starts at its minimum, an index past the end is refused, an order is unsubmittable until every offered object is in it exactly once, and every indexed choice reports the row it picked so one renderer row serves all of them. Answering one mode's question in another mode's shape yields `None` rather than an action.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn x_is_clamped_to_the_range_the_engine_offered() {
    let mut i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 50,
        reason: baylee_engine::choice::NumberPrompt::X,
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
        reason: baylee_engine::choice::NumberPrompt::X,
    });
    assert_eq!(i.number(), 3);
}

/// Report 20: an X question the engine bounds only by the number type is
/// capped at what the seat's mana pays; the cap shows in the question, in
/// what a tap on a bigger number answers, and in what is sent.
#[test]
fn a_cap_lowers_the_top_of_an_x_question_and_what_is_answered() {
    let mut i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: u32::MAX,
        reason: baylee_engine::choice::NumberPrompt::X,
    });
    i.cap_number(3);
    let Pending::ChooseNumber { min, max, .. } = i.pending() else {
        panic!("still a number question");
    };
    assert_eq!((*min, *max), (0, 3));
    assert_eq!(i.set_number(10), 3);
    assert_eq!(i.confirm(), Some(PlayerAction::ChooseNumber(3)));
}

/// Report 20: a number already chosen above the cap comes down with it.
#[test]
fn a_cap_pulls_a_number_already_chosen_down_to_it() {
    let mut i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 100,
        reason: baylee_engine::choice::NumberPrompt::X,
    });
    assert_eq!(i.set_number(40), 40);
    i.cap_number(5);
    assert_eq!(i.number(), 5);
    assert_eq!(i.confirm(), Some(PlayerAction::ChooseNumber(5)));
}

/// Report 20: the cap never goes below the engine's own minimum, and never
/// raises the engine's maximum.
#[test]
fn a_cap_below_the_minimum_leaves_the_maximum_at_the_minimum_and_never_raises_it() {
    let mut i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 4,
        max: 9,
        reason: baylee_engine::choice::NumberPrompt::X,
    });
    i.cap_number(1);
    let Pending::ChooseNumber { min, max, .. } = i.pending() else {
        panic!("still a number question");
    };
    assert_eq!((*min, *max), (4, 4));
    assert_eq!(i.set_number(9), 4);

    let mut i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 6,
        reason: baylee_engine::choice::NumberPrompt::X,
    });
    i.cap_number(50);
    let Pending::ChooseNumber { max, .. } = i.pending() else {
        panic!("still a number question");
    };
    assert_eq!(*max, 6, "a cap only lowers");
    assert_eq!(i.set_number(u32::MAX), 6);
}

/// Report 20: only an X is the player's own to bound; the engine's range
/// for any other number stands.
#[test]
fn a_cap_leaves_a_number_that_is_not_x_alone() {
    let mut i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 80,
        reason: baylee_engine::choice::NumberPrompt::Counters {
            target: obj(1),
            kind: baylee_engine::object::CounterKind::Plus {
                power: 1,
                toughness: 0,
            },
        },
    });
    i.cap_number(3);
    let Pending::ChooseNumber { max, .. } = i.pending() else {
        panic!("still a number question");
    };
    assert_eq!(*max, 80);
    assert_eq!(i.set_number(60), 60);
    assert_eq!(i.confirm(), Some(PlayerAction::ChooseNumber(60)));
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

#[test]
fn a_card_type_choice_is_answerable() {
    let options = vec![
        baylee_core::types::TypeSet::INSTANT,
        baylee_core::types::TypeSet::CREATURE,
        baylee_core::types::TypeSet::SORCERY,
    ];
    let mut i = interaction(Pending::ChooseCardType {
        player: me(),
        options: options.clone(),
    });
    assert!(!i.can_confirm());
    assert!(!i.choose_index(options.len()));
    assert!(i.choose_index(1));
    assert_eq!(i.chosen_index(), Some(1));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseCardType(
            baylee_core::types::TypeSet::CREATURE
        ))
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
        can_take: true,
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

#[test]
fn inspecting_a_hand_needs_only_confirmation_and_never_selects_cards() {
    for options in [vec![], vec![obj(1), obj(2)]] {
        let mut i = interaction(Pending::ChooseCards {
            player: me(),
            options,
            min: 0,
            max: 0,
            prompt: ChoicePrompt::LookAtHand,
            total: None,
        });
        assert!(i.can_confirm());
        assert!(!i.is_selectable(obj(1)));
        assert_eq!(i.toggle(obj(1)), SelectionOutcome::Rejected);
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::ChooseObjects { objects: vec![] })
        );
        assert_eq!(
            i.prompt().headline(Lang::En, Turn::Mine, None, false),
            "Look at the hand, then confirm when finished"
        );
        assert_eq!(
            i.prompt().headline(Lang::De, Turn::Mine, None, false),
            "Sieh dir die Hand an und bestätige, wenn du fertig bist"
        );
    }
}

#[test]
fn choosing_colors_is_described_as_a_resolution_choice_and_submits_the_offered_index() {
    use baylee_core::{
        color::{Color, ColorSet},
        mana::ManaCost,
    };
    use baylee_engine::choice::{CastModeDesc, CastModeKind};
    let mut i = interaction(Pending::ChooseCastMode {
        player: me(),
        object: obj(1),
        options: vec![CastModeDesc {
            index: 0,
            kind: CastModeKind::Colors(ColorSet::of(Color::Blue)),
            cost: ManaCost::ZERO,
        }],
    });
    for lang in [Lang::En, Lang::De] {
        assert_eq!(
            i.prompt().headline(lang, Turn::Mine, None, false),
            Phrase::ChooseColours.text(lang)
        );
    }
    assert!(!i.choose_index(1));
    assert!(i.choose_index(0));
    assert_eq!(i.confirm(), Some(PlayerAction::ChooseMode(0)));
}

#[test]
fn counter_kind_choice_uses_named_rows_and_preserves_large_indices() {
    use baylee_engine::choice::{CastModeDesc, CastModeKind};
    let options = (0_u16..300)
        .map(|kind| CastModeDesc {
            index: u32::from(kind),
            kind: CastModeKind::Counter(baylee_cards_dsl::CounterKind::Custom(kind)),
            cost: baylee_core::mana::ManaCost::ZERO,
        })
        .collect();
    let mut i = interaction(Pending::ChooseCastMode {
        player: me(),
        object: obj(1),
        options,
    });
    for lang in [Lang::En, Lang::De] {
        assert_eq!(
            i.prompt().headline(lang, Turn::Mine, None, false),
            Phrase::ChooseCounterKind.text(lang)
        );
    }
    assert!(i.choose_index(299));
    assert_eq!(i.confirm(), Some(PlayerAction::ChooseMode(299)));
}

#[test]
fn named_custom_counters_remain_distinguishable_in_the_counter_chooser() {
    for lang in [Lang::En, Lang::De] {
        let names: Vec<_> = baylee_cards_dsl::counters::ASSIGNED
            .iter()
            .map(|(name, kind)| {
                let label = counter_label(*kind, lang);
                assert!(label.contains(name));
                label
            })
            .collect();
        let unique: std::collections::BTreeSet<_> = names.iter().collect();
        assert_eq!(unique.len(), names.len());
    }
}
