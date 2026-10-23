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
    assert_eq!(
        engine.state().players[1].life,
        20,
        "no infection counters yet: the upkeep ability dealt 0"
    );
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

/// At a table of three the upkeep ability is off: it would let the Vermin hit an opponent it never
/// damaged ("target opponent previously dealt damage by it" is not in the DSL), so it does nothing
/// there. The counter on combat damage still works.
#[test]
fn diseased_vermin_upkeep_ability_is_off_with_two_opponents() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, swamp(), 3)
        .hand(0, &[diseased_vermin()])
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, diseased_vermin());
    pass_until(&mut engine, stack_is_empty);
    let vermin = on_battlefield(&engine, p0, diseased_vermin()).expect("Diseased Vermin enters");

    // P0's second turn is the fourth of the game: the Vermin attacks P1 and connects. Its upkeep
    // ability asks for no target on the way (`pass_until` stops with a panic on a question it does
    // not know).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p2);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
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

    // P0's third turn: one infection counter, two opponents, no upkeep damage to either.
    reach_their_main_phase(&mut engine, p2);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(engine.state().players[2].life, 20);
}
