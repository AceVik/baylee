//! `cards/creatures/mv_2/saprazzan_heir.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "8685ebfd-d04d-47f5-a919-e118db1e2276"
fn saprazzan_heir() -> CardIndex {
    card_index("8685ebfd-d04d-47f5-a919-e118db1e2276")
}

/// `Saprazzan Heir` (`Coverage::Generated`):
/// "Whenever this creature becomes blocked, you may draw three cards."
///
/// Verifies that when `Saprazzan Heir` becomes blocked, its controller is asked
/// whether to draw three cards and draws three when accepting.
#[test]
fn saprazzan_heir_lets_controller_draw_three_cards_when_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9607, island())
        .battlefield(0, &[saprazzan_heir()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let heir = on_battlefield(&engine, p0, saprazzan_heir()).expect("P0 Saprazzan Heir");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(heir, Defender::Player(p1))],
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
                blockers: vec![(bears, heir)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 3,
        "Saprazzan Heir draws 3 cards when its controller accepts"
    );
}
