//! `cards/creatures/mv_2/mogg_jailer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Mogg Jailer — {1}{R} Creature — Goblin 2/2
/// "This creature can't attack if defending player controls an untapped creature with power 2 or less."
#[test]
fn mogg_jailer_cannot_attack_when_defender_controls_untapped_small_creature() {
    let jailer = card_index("423659f1-1b48-4a2b-a1dc-f39fd62c6b6a");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[jailer, index::HILL_GIANT])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let jailer_id = on_battlefield(&engine, p0, jailer).expect("Mogg Jailer on board");
    let giant_id = on_battlefield(&engine, p0, index::HILL_GIANT).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(attackers.contains(&giant_id));
    assert!(
        !attackers.contains(&jailer_id),
        "Mogg Jailer cannot attack p1 while p1 controls untapped Grizzly Bears (power 2)"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(jailer_id, Defender::Player(p1))],
                },
            )
            .is_err()
    );
}

#[test]
fn mogg_jailer_can_attack_when_defender_only_controls_creature_with_power_three_or_more() {
    let jailer = card_index("423659f1-1b48-4a2b-a1dc-f39fd62c6b6a");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[jailer])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let jailer_id = on_battlefield(&engine, p0, jailer).expect("Mogg Jailer on board");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(
        attackers.contains(&jailer_id),
        "Mogg Jailer can attack when defender only controls a power-3 creature"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(jailer_id, Defender::Player(p1))],
            },
        )
        .unwrap();
}
