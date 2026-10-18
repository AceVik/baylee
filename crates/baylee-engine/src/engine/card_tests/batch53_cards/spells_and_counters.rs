#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn stock_up_looks_at_top_five_takes_two_to_hand_and_orders_rest_on_bottom() {
    let mut e = prepared(&[index::STOCK_UP], &[], &[]);
    let hand_before = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    let lib_before = library_size(&e, P0);

    cast(&mut e, P0, index::STOCK_UP);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::PutIntoHand,
                ..
            }
        )
    });

    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending().clone()
    else {
        panic!("expected ChooseCards for Stock Up, got {:?}", e.pending());
    };
    assert_eq!(options.len(), 5, "Stock Up looks at the top 5 cards");
    assert_eq!((min, max), (2, 2), "Stock Up puts 2 of them into your hand");

    choose_objects(&mut e, vec![options[0], options[1]]);

    // Then put the rest on the bottom of your library in any order.
    assert!(matches!(
        e.pending(),
        Pending::Arrange {
            prompt: ArrangePrompt::Order,
            ..
        }
    ));
    e.apply(
        P0,
        PlayerAction::Arrange {
            piles: vec![vec![options[2], options[3], options[4]]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    // Spent Stock Up (-1), put 2 into hand (+2) => net +1 in hand.
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1
    );
    assert_eq!(library_size(&e, P0), lib_before - 2);
}

#[test]
fn nuclear_fallout_gives_all_creatures_twice_minus_x_minus_x_and_each_player_x_rad() {
    let mut e = prepared(
        &[index::NUCLEAR_FALLOUT],
        &[index::HILL_GIANT],
        &[index::GRIZZLY_BEARS],
    );

    cast(&mut e, P0, index::NUCLEAR_FALLOUT);
    assert!(matches!(
        e.pending(),
        Pending::ChooseNumber { player: P0, .. }
    ));
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut e, stack_is_empty);

    // X = 2 => -4/-4 to each creature: both Hill Giant (3/3) and Grizzly Bears (2/2) die!
    assert!(on_battlefield(&e, P0, index::HILL_GIANT).is_none());
    assert!(on_battlefield(&e, P1, index::GRIZZLY_BEARS).is_none());
    assert_eq!(e.state().players[0].rad, 2);
    assert_eq!(e.state().players[1].rad, 2);
}

#[test]
fn countersculpt_counters_target_creature_spell_and_creates_flying_spirit_token() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::GRIZZLY_BEARS])
        .hand(1, &[index::COUNTERSCULPT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);

    let bears = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: bears })
        .unwrap();
    let spell_on_stack = *e.state().zones.list(ZoneLocation::Stack).last().unwrap();

    // P0 passes priority so P1 can respond with Countersculpt.
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    mana(&mut e);
    let sculpt = in_hand(&e, P1, index::COUNTERSCULPT).unwrap();
    e.apply(P1, PlayerAction::CastSpell { card: sculpt })
        .unwrap();
    targets(&mut e, vec![spell_on_stack], vec![]);
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_none());
    let jace_token = e
        .state()
        .battlefield_seen()
        .find(|id| {
            e.state().object(*id).is_some_and(|o| {
                o.controller == P1
                    && o.token.is_some()
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::planeswalker::JACE)
            })
        })
        .expect("Countersculpt creates a blue Jace planeswalker token via Empower Jace 1");
    assert_eq!(
        e.state()
            .object(jace_token)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        1,
        "Jace token enters at 0 base loyalty and Empower Jace 1 puts 1 loyalty counter on it"
    );
}

#[test]
fn legolas_s_quick_reflexes_untaps_grants_reach_hexproof_and_tap_damage_trigger() {
    let mut e = prepared(
        &[index::LEGOLAS_S_QUICK_REFLEXES],
        &[index::HILL_GIANT],
        &[index::GRIZZLY_BEARS],
    );
    let giant = on_battlefield(&e, P0, index::HILL_GIANT).unwrap();
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();

    // Tap Hill Giant first so UntapTarget is verified.
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(giant)
        .unwrap()
        .status
        .insert(Status::TAPPED);

    cast(&mut e, P0, index::LEGOLAS_S_QUICK_REFLEXES);
    targets(&mut e, vec![giant], vec![]);
    pass_until(&mut e, stack_is_empty);

    let obj = e.state().object(giant).unwrap();
    assert!(
        !obj.status.contains(Status::TAPPED),
        "Hill Giant was untapped"
    );
    assert!(obj.characteristics().keywords.contains(KeywordSet::REACH));
    assert!(
        obj.characteristics()
            .keywords
            .contains(KeywordSet::HEXPROOF)
    );

    // Attack with Hill Giant so it becomes tapped, firing the granted trigger!
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(giant, Defender::Player(P1))],
        },
    )
    .unwrap();

    // The granted trigger asks for up to one target creature!
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { min, max, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!((*min, *max), (0, 1), "up to one target creature");
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    // Hill Giant has power 3, so Grizzly Bears (2/2) takes 3 damage and dies!
    assert!(on_battlefield(&e, P1, index::GRIZZLY_BEARS).is_none());
}
