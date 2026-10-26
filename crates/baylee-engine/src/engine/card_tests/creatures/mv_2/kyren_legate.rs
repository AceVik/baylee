//! `cards/creatures/mv_2/kyren_legate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "41b46aa5-6091-4bc4-8d17-d2c6b5fe1c5e"
fn kyren_legate() -> CardIndex {
    card_index("41b46aa5-6091-4bc4-8d17-d2c6b5fe1c5e")
}

/// `Kyren Legate` (`Coverage::Partial`):
/// "Haste. (If an opponent controls a Plains and you control a Mountain, you may cast this spell without paying its mana cost.)"
///
/// Verifies that `Kyren Legate` enters with `KeywordSet::HASTE` and can attack immediately.
#[test]
fn kyren_legate_attacks_immediately_with_haste() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9405, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[kyren_legate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, kyren_legate());
    pass_until(&mut engine, stack_is_empty);

    let legate = on_battlefield(&engine, p0, kyren_legate()).expect("Kyren Legate on battlefield");
    assert!(keywords(&engine, legate).contains(KeywordSet::HASTE));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(legate, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 19);
}
