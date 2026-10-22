//! `cards/creatures/mv_3/diseased_vermin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn diseased_vermin() -> CardIndex {
    card_index("4d18bea4-c9b2-4fc7-948f-2583589994d7")
}

/// `Diseased Vermin` ({2}{B}, 1/1 Creature — Rat, `Coverage::Partial`) puts an
/// infection counter on itself whenever it deals combat damage to a player, and
/// at the beginning of your upkeep deals X damage to target opponent, where X
/// is the number of infection counters on it.
#[test]
fn diseased_vermin_gains_infection_counter_on_combat_damage_and_pings_opponent_on_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 3])
        .hand(0, &[diseased_vermin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, diseased_vermin());
    pass_until(&mut engine, stack_is_empty);
    let vermin = on_battlefield(&engine, p0, diseased_vermin()).expect("Diseased Vermin enters");
    assert_eq!(pt(&engine, vermin), (1, 1));

    // Advance to P0's next combat and attack P1 unblocked (answering the upkeep trigger target along the way).
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(vermin, Defender::Player(p1))],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(
        counters_on(&engine, vermin, baylee_cards_dsl::counters::INFECTION),
        1
    );

    // On P0's next upkeep, Diseased Vermin deals 1 damage to target opponent P1.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 18);
}
