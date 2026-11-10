//! `cards/instants/mv_1/awe_strike.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn awe_strike() -> CardIndex {
    card_index("994bcde3-4a76-403d-a6e1-88609a13a99c")
}

/// Awe Strike — {W} Instant
/// "The next time target creature would deal damage this turn, prevent that damage. You gain life equal to the damage prevented this way."
#[test]
fn awe_strike_prevents_next_damage_from_target_creature_and_gains_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[awe_strike()])
        .battlefield(0, &[plains()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    // Advance to P1's turn so P1 can attack with Hill Giant (3/3).
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p0))],
            },
        )
        .unwrap();

    // Cast Awe Strike targeting the attacking Hill Giant before combat damage.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_from_hand(&mut engine, p0, awe_strike());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // The 3 combat damage from Hill Giant was prevented and P0 gained 3 life.
    assert_eq!(engine.state().players[0].life, 23);
}
