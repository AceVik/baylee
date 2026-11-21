//! `cards/artifacts/mv_4/razor_pendulum.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn razor_pendulum() -> CardIndex {
    card_index("bf7c16db-01c9-41ed-bda6-c5326198ce77")
}

fn spark_spray() -> CardIndex {
    card_index("c37f1921-7ac3-4185-8579-1dfdfe647ea2")
}

fn burn_self(engine: &mut Engine<RegistryLookup>, seat: PlayerId, spell: CardIndex) {
    cast_from_hand(engine, seat, spell);
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![seat],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// Razor Pendulum (bf7c16db-01c9-41ed-bda6-c5326198ce77) — {4} Artifact:
/// "At the beginning of each player's end step, if that player has 5 or less
///  life, this artifact deals 2 damage to that player."
///
/// Plays down to 6 life on p0's turn (does NOT trigger), 5 life on p1's turn
/// (triggers for 2 -> 3), then 5 life on p0's next turn (triggers for 2 -> 3).
#[test]
fn razor_pendulum_hits_at_five_life_and_skips_six_for_each_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                razor_pendulum(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(
            0,
            &[
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
                spark_spray(),
                spark_spray(),
                spark_spray(),
            ],
        )
        .battlefield(
            1,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(
            1,
            &[
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 1. Bring p0 from 20 down to 6 life (4x Bolt = 12, 2x Spark Spray = 2).
    for _ in 0..4 {
        burn_self(&mut engine, p0, lightning_bolt());
    }
    for _ in 0..2 {
        burn_self(&mut engine, p0, spark_spray());
    }
    assert_eq!(life_of(&engine, p0), 6);

    // Cross p0's end step into p1's turn: at 6 life, Razor Pendulum must NOT trigger.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        life_of(&engine, p0),
        6,
        "at 6 life (> 5), Razor Pendulum does not deal damage"
    );

    // 2. Bring p1 from 20 down to 5 life (5x Bolt = 15).
    for _ in 0..5 {
        burn_self(&mut engine, p1, lightning_bolt());
    }
    assert_eq!(life_of(&engine, p1), 5);

    // Cross p1's end step into p0's turn: p1 takes 2 damage (5 -> 3).
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        life_of(&engine, p1),
        3,
        "at 5 life, Razor Pendulum deals 2 damage to the opponent on their end step"
    );

    // 3. Bring p0 from 6 down to 5 life (1x Spark Spray) and cross p0's end step.
    burn_self(&mut engine, p0, spark_spray());
    assert_eq!(life_of(&engine, p0), 5);
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(
        life_of(&engine, p0),
        3,
        "at 5 life, Razor Pendulum deals 2 damage to you on your end step"
    );
}
