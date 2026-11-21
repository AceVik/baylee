//! `cards/artifacts/mv_2/fluctuator.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fluctuator() -> CardIndex {
    card_index("5ed0c7cc-9969-4387-90f8-3084dd18026b")
}

fn miscalculation() -> CardIndex {
    card_index("9078db26-3a54-4a11-9aae-e45c38da7226")
}

fn scion_of_darkness() -> CardIndex {
    card_index("bd940cab-d23a-44bf-ab96-8aa1dcd3f0ff")
}

fn spark_spray() -> CardIndex {
    card_index("c37f1921-7ac3-4185-8579-1dfdfe647ea2")
}

/// Fluctuator (5ed0c7cc-9969-4387-90f8-3084dd18026b) — {2} Artifact:
/// "Cycling abilities you activate cost {2} less to activate."
///
/// Reduces your Cycling {2} to {0} and Cycling {3} to {1}, does not reduce
/// colored Cycling {R}, and does not reduce the opponent's Cycling {2}.
#[test]
fn fluctuator_reduces_your_generic_cycling_costs_by_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fluctuator(), forest()])
        .hand(0, &[miscalculation(), scion_of_darkness(), spark_spray()])
        .battlefield(1, &[island()])
        .hand(1, &[miscalculation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let misc = in_hand(&engine, p0, miscalculation()).unwrap();
    let scion = in_hand(&engine, p0, scion_of_darkness()).unwrap();
    let spray = in_hand(&engine, p0, spark_spray()).unwrap();

    // With 0 mana floating, Cycling {2} is free (0), Cycling {3} needs {1},
    // and Cycling {R} still needs {R}.
    let offer = priority_offer(&engine);
    assert!(
        offer.abilities.contains(&(misc, 0)),
        "Cycling {{2}} is free with Fluctuator"
    );
    assert!(
        !offer.abilities.contains(&(scion, 0)),
        "Cycling {{3}} still needs {{1}} with Fluctuator"
    );
    assert!(
        !offer.abilities.contains(&(spray, 0)),
        "Cycling {{R}} has no generic part to reduce"
    );

    // Cycle Miscalculation for {0}.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: misc,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(misc).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "discarded 1, drew 1"
    );

    // Tap 1 Forest: Cycling {3} on Scion of Darkness costs {1}; Cycling {R} still illegal.
    tap_all_mana(&mut engine, p0);
    let offer = priority_offer(&engine);
    assert!(offer.abilities.contains(&(scion, 0)));
    assert!(
        !offer.abilities.contains(&(spray, 0)),
        "green mana cannot pay Cycling {{R}}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: scion,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(scion).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // Opponent with only 1 Island still cannot cycle Miscalculation (Cycling {2}).
    reach_their_main_phase(&mut engine, p1);
    let their_misc = in_hand(&engine, p1, miscalculation()).unwrap();
    tap_all_mana(&mut engine, p1);
    assert!(
        !priority_offer(&engine).abilities.contains(&(their_misc, 0)),
        "Fluctuator only discounts cycling abilities you activate"
    );
}
