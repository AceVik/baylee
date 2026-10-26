//! Tests for Ignoble Soldier.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::hill_giant;

// oracle_id = "52a61983-53e2-440f-af03-cebe57b102f2"
fn ignoble_soldier() -> CardIndex {
    card_index("52a61983-53e2-440f-af03-cebe57b102f2")
}

/// Whenever Ignoble Soldier becomes blocked, prevent all combat damage that
/// would be dealt by it this turn.
#[test]
fn ignoble_soldier_deals_no_combat_damage_when_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), ignoble_soldier()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let soldier = on_battlefield(&engine, p0, ignoble_soldier()).expect("Ignoble Soldier");
    let blocker = on_battlefield(&engine, p1, hill_giant()).expect("Hill Giant");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(soldier, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, soldier)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    // Ignoble Soldier (3/1) died to Hill Giant (3/3), and Hill Giant took 0 combat damage.
    assert!(on_battlefield(&engine, p0, ignoble_soldier()).is_none());
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("Hill Giant survives");
    assert_eq!(engine.state().object(giant).unwrap().damage, 0);
}
