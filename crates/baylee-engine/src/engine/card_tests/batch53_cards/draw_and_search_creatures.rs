#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn wan_shi_tong_librarian_etb_x_counters_half_x_draw_and_opponent_search_trigger() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::WAN_SHI_TONG_LIBRARIAN])
        .hand(1, &[index::GAMBLE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);

    let hand_before = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    cast(&mut e, P0, index::WAN_SHI_TONG_LIBRARIAN);
    e.apply(P0, PlayerAction::ChooseNumber(4)).unwrap();
    pass_until(&mut e, stack_is_empty);

    let owl = on_battlefield(&e, P0, index::WAN_SHI_TONG_LIBRARIAN).unwrap();
    assert_eq!(
        e.state()
            .object(owl)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        4,
        "entered with X=4 +1/+1 counters"
    );
    // Cast Wan Shi Tong (-1) and drew floor(4/2) = 2 cards (+2) => net +1 in hand.
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1
    );

    // Advance to P1's main phase and have P1 cast Gamble (searches library) -> Wan Shi Tong triggers!
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    cast(&mut e, P1, index::GAMBLE);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    choose_objects(&mut e, vec![options[0]]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state()
            .object(owl)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        5,
        "Wan Shi Tong gained a +1/+1 counter when opponent searched library"
    );
}

#[test]
fn faerie_mastermind_triggers_on_opponents_second_draw_in_a_turn() {
    let mut e = prepared(&[], &[index::FAERIE_MASTERMIND], &[]);

    // Advance to P1's main phase (P1 has already drawn 1 card in their draw step).
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });

    // P0 activates Faerie Mastermind's {3}{U}: Each player draws a card.
    // That makes P1 draw their 2nd card of the turn, triggering Faerie Mastermind for an extra draw for P0!
    let p0_hand_before = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    e.apply(P1, PlayerAction::PassPriority).unwrap();
    mana(&mut e);
    let fm = on_battlefield(&e, P0, index::FAERIE_MASTERMIND).unwrap();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: fm,
            ability_index: 1,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        p0_hand_before + 2,
        "P0 drew 1 from activated ability + 1 from Faerie Mastermind's trigger"
    );
}

#[test]
fn hullbreacher_replaces_opponent_extra_draws_with_treasure_tokens() {
    let mut e = prepared(&[], &[index::HULLBREACHER, index::FAERIE_MASTERMIND], &[]);

    // On P0's turn, any draw by P1 is outside P1's draw step!
    // Activate Faerie Mastermind's {3}{U}: Each player draws a card.
    let p1_hand_before = e.state().zones.list(ZoneLocation::Hand(P1)).len();
    activate_by_card(&mut e, P0, index::FAERIE_MASTERMIND, 1);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand_before,
        "Hullbreacher prevented P1 from drawing"
    );
    assert!(
        e.state().battlefield_seen().any(|id| {
            e.state().object(id).is_some_and(|o| {
                o.controller == P0
                    && o.token.is_some()
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::artifact::TREASURE)
            })
        }),
        "Hullbreacher created a Treasure token for P0 instead"
    );
}

#[test]
fn tamiyo_inquisitive_student_investigates_on_attack_and_transforms_on_third_draw() {
    let mut e = prepared(
        &[index::EMERITUS_OF_IDEATION, index::LIGHTNING_BOLT],
        &[index::TAMIYO_INQUISITIVE_STUDENT],
        &[],
    );

    // Attack with Tamiyo -> she investigates (creates a Clue token).
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    let tamiyo = on_battlefield(&e, P0, index::TAMIYO_INQUISITIVE_STUDENT).unwrap();
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(tamiyo, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(
        e.state().battlefield_seen().any(|id| {
            e.state().object(id).is_some_and(|o| {
                o.controller == P0
                    && o.token.is_some()
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::artifact::CLUE)
            })
        }),
        "Tamiyo's attack trigger created a Clue token"
    );

    // Cast Emeritus of Ideation and use its prepared Ancestral Recall to draw 3 cards in P0's turn!
    mana(&mut e);
    cast(&mut e, P0, index::EMERITUS_OF_IDEATION);
    pass_until(&mut e, stack_is_empty);
    let emeritus = on_battlefield(&e, P0, index::EMERITUS_OF_IDEATION).unwrap();
    mana(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: emeritus,
            ability_index: crate::choice::PREPARED_CAST,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    pass_until(&mut e, stack_is_empty);

    // Tamiyo exiled herself and returned transformed on face 1 (Tamiyo, Seasoned Scholar)!
    let walker = on_battlefield(&e, P0, index::TAMIYO_INQUISITIVE_STUDENT).unwrap();
    assert_eq!(
        e.state().object(walker).unwrap().face_index,
        1,
        "Tamiyo transformed into Tamiyo, Seasoned Scholar"
    );
    assert!(
        e.state()
            .object(walker)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::PLANESWALKER)
    );

    // Activate Tamiyo, Seasoned Scholar's +2 loyalty ability (ability 0).
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: walker,
            ability_index: 0,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(walker)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        4
    );
}

#[test]
fn emeritus_of_ideation_enters_prepared_casts_ancestral_recall_and_reprepares_on_attack() {
    let mut e = prepared(
        &[
            index::EMERITUS_OF_IDEATION,
            index::FOREST,
            index::FOREST,
            index::FOREST,
            index::FOREST,
            index::FOREST,
        ],
        &[index::PSYCHIC_FROG],
        &[],
    );

    cast(&mut e, P0, index::EMERITUS_OF_IDEATION);
    pass_until(&mut e, stack_is_empty);
    let emeritus = on_battlefield(&e, P0, index::EMERITUS_OF_IDEATION).unwrap();
    assert!(
        e.state()
            .object(emeritus)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Prepared),
        "Emeritus of Ideation enters prepared"
    );

    // Cast prepared copy of Ancestral Recall targeting P0 -> P0 draws 3 cards and Emeritus becomes unprepared.
    let hand_before = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    mana(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: emeritus,
            ability_index: crate::choice::PREPARED_CAST,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 3,
        "Ancestral Recall copy drew 3 cards"
    );
    assert!(
        !e.state()
            .object(emeritus)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Prepared),
        "Emeritus became unprepared after casting its prepared spell"
    );

    // Discard 8 cards to Psychic Frog to populate P0's graveyard with 8 cards.
    for _ in 0..8 {
        let card = e.state().zones.list(ZoneLocation::Hand(P0))[0];
        activate_by_card(&mut e, P0, index::PSYCHIC_FROG, 1);
        choose_objects(&mut e, vec![card]);
        pass_until(&mut e, stack_is_empty);
    }

    // Next turn, attack with Emeritus of Ideation -> exile 8 cards from graveyard -> it becomes prepared again!
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(emeritus, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::YesNo { player: P0, .. })
    });
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    choose_objects(&mut e, options[..8].to_vec());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        e.state()
            .object(emeritus)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Prepared),
        "Emeritus of Ideation became prepared again when it attacked and exiled 8 graveyard cards"
    );
}
