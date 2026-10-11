//! Cause, known characteristics and multiplayer discard timing.
#[allow(clippy::wildcard_imports)]
use super::*;

fn thoughtseize(e: &mut Engine<RegistryLookup>, target: PlayerId) {
    cast_with_floating(e, P0, index::THOUGHTSEIZE);
    e.apply(P0, PlayerAction::ChoosePlayer(target)).unwrap();
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseCards { .. }));
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![options[0]],
        },
    )
    .unwrap();
}

#[test]
fn discarded_psychic_purge_punishes_only_the_opponent_causing_its_discard() {
    for academy in [false, true] {
        let mut e = prepared(
            Duel::table(SEED, index::FOREST, 3)
                .hand(0, &[index::THOUGHTSEIZE])
                .hand(1, &[index::PSYCHIC_PURGE])
                .battlefield(
                    1,
                    if academy {
                        &[index::NEPHALIA_ACADEMY]
                    } else {
                        &[]
                    },
                ),
        );
        let purge = in_hand(&e, P1, index::PSYCHIC_PURGE).unwrap();
        thoughtseize(&mut e, P1);
        if academy {
            e.apply(
                P1,
                PlayerAction::Arrange {
                    piles: vec![vec![], vec![purge]],
                },
            )
            .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().players[0].life,
            13,
            "two life to Thoughtseize plus five to Purge"
        );
        assert_eq!(e.state().players[1].life, 20);
        assert_eq!(
            e.state().players[2].life,
            20,
            "not every opponent loses life"
        );
        assert_eq!(
            e.state().object(purge).unwrap().zone,
            if academy {
                Zone::Library
            } else {
                Zone::Graveyard
            }
        );
    }
}

#[test]
fn discarded_academy_and_leng_allow_a_per_card_public_or_hidden_choice() {
    for reveal in [false, true] {
        let mut e = prepared(
            Duel::new(SEED, index::FOREST)
                .hand(0, &[index::THOUGHTSEIZE])
                .hand(1, &[index::PSYCHIC_PURGE])
                .battlefield(1, &[index::NEPHALIA_ACADEMY, index::LIBRARY_OF_LENG]),
        );
        let purge = in_hand(&e, P1, index::PSYCHIC_PURGE).unwrap();
        thoughtseize(&mut e, P1);
        e.apply(
            P1,
            PlayerAction::Arrange {
                piles: vec![vec![], vec![purge]],
            },
        )
        .unwrap();
        assert!(matches!(
            e.pending(),
            Pending::ChooseCards {
                player: P1,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::RevealDiscard,
                ..
            }
        ));
        e.apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: if reveal { vec![purge] } else { vec![] },
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, if reveal { 13 } else { 18 });
        assert!(e.journal().entries().iter().any(|entry| matches!(entry.event, GameEvent::Discarded { object, known, .. } if object == purge && known == reveal)));
    }
}

#[test]
fn discarded_academy_does_not_replace_your_own_spell_or_an_activation_cost() {
    let mut e = ready(
        &[index::THOUGHTSEIZE, index::PSYCHIC_PURGE],
        &[index::NEPHALIA_ACADEMY],
    );
    thoughtseize(&mut e, P0);
    assert!(!matches!(e.pending(), Pending::Arrange { .. }));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 18);
    let mut e = ready(
        &[index::PSYCHIC_PURGE],
        &[
            index::CORAL_HELM,
            index::NEPHALIA_ACADEMY,
            index::GRIZZLY_BEARS,
        ],
    );
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    activate(&mut e, P0, index::CORAL_HELM, 0);
    aim_at(&mut e, P0, bear);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 20);
    assert_eq!(e.state().players[0].life, 20);
    assert!(in_graveyard(&e, P0, index::PSYCHIC_PURGE).is_some());
}

#[test]
fn discarded_aclazotz_waits_for_all_choices_draws_for_empty_hands_and_sees_known_lands() {
    let p2 = PlayerId::new(2);
    let p3 = PlayerId::new(3);
    let mut e = prepared(
        Duel::table(SEED, index::FOREST, 4)
            .battlefield(0, &[index::ACLAZOTZ_DEEPEST_BETRAYAL])
            .hand(1, &[index::FOREST])
            .hand(2, &[index::GRIZZLY_BEARS]),
    );
    let bat = on_battlefield(&e, P0, index::ACLAZOTZ_DEEPEST_BETRAYAL).unwrap();
    let library = library_size(&e, P0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bat, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let land = in_hand(&e, P1, index::FOREST).unwrap();
    e.apply(
        P1,
        PlayerAction::ChooseObjects {
            objects: vec![land],
        },
    )
    .unwrap();
    assert_eq!(
        e.state().object(land).unwrap().zone,
        Zone::Hand,
        "the next player cannot inspect an earlier discard"
    );
    let bear = in_hand(&e, p2, index::GRIZZLY_BEARS).unwrap();
    e.apply(
        p2,
        PlayerAction::ChooseObjects {
            objects: vec![bear],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        library_size(&e, P0),
        library - 1,
        "only the empty-handed opponent caused a draw"
    );
    assert!(e.state().zones.list(ZoneLocation::Hand(p3)).is_empty());
    let bats = tokens_of(&e, P0);
    assert_eq!(bats.len(), 1);
    assert_eq!(pt(&e, bats[0]), (1, 1));
    assert!(keywords(&e, bats[0]).contains(KeywordSet::FLYING));
}

#[test]
fn discarded_aclazotz_observes_land_costs_but_not_hidden_discard_characteristics() {
    for hidden in [false, true] {
        let mut e = prepared(
            Duel::new(SEED, index::FOREST)
                .hand(0, &[index::FOREST])
                .battlefield(0, &[index::CORAL_HELM, index::GRIZZLY_BEARS])
                .battlefield(1, &[index::ACLAZOTZ_DEEPEST_BETRAYAL]),
        );
        let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
        activate(&mut e, P0, index::CORAL_HELM, 0);
        aim_at(&mut e, P0, bear);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(tokens_of(&e, P1).len(), 1);
        // Cost above always counts; now an actual opponent's spell discards through Leng.
        let mut field = vec![index::LIBRARY_OF_LENG];
        if !hidden {
            field.push(index::NEPHALIA_ACADEMY);
        }
        let mut e = prepared(
            Duel::new(SEED, index::FOREST)
                .hand(0, &[index::WHEEL_OF_FORTUNE])
                .battlefield(0, &[index::ACLAZOTZ_DEEPEST_BETRAYAL])
                .hand(1, &[index::FOREST])
                .battlefield(1, &field),
        );
        let land = in_hand(&e, P1, index::FOREST).unwrap();
        cast_with_floating(&mut e, P0, index::WHEEL_OF_FORTUNE);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
        e.apply(
            P1,
            PlayerAction::Arrange {
                piles: vec![vec![], vec![land]],
            },
        )
        .unwrap();
        if !hidden {
            e.apply(
                P1,
                PlayerAction::ChooseObjects {
                    objects: vec![land],
                },
            )
            .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(tokens_of(&e, P0).len(), usize::from(!hidden));
    }
}

#[test]
fn discarded_aclazotz_returns_tapped_transformed_and_requires_a_small_hand_to_turn_back() {
    let mut e = ready(
        &[index::HERO_S_DOWNFALL],
        &[index::ACLAZOTZ_DEEPEST_BETRAYAL],
    );
    let bat = on_battlefield(&e, P0, index::ACLAZOTZ_DEEPEST_BETRAYAL).unwrap();
    cast_with_floating(&mut e, P0, index::HERO_S_DOWNFALL);
    aim_at(&mut e, P0, bat);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, bat));
    assert_eq!(
        e.state().object(bat).unwrap().characteristics().types,
        TypeSet::LAND
    );
    e.dev_state_mut(P0).unwrap().set_tapped(bat, false);
    e.dev_state_mut(P0).unwrap().draw_cards(P0, 2);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        unreachable!()
    };
    assert!(
        !legal.abilities.contains(&(bat, 1)),
        "every player has at least two cards"
    );
    let card = e.state().zones.list(ZoneLocation::Hand(P0))[0];
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(
            card,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    activate(&mut e, P0, index::ACLAZOTZ_DEEPEST_BETRAYAL, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, bat), (4, 4));
    assert!(is_tapped(&e, bat));
}
