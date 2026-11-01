//! `cards/creatures/mv_3/saprazzan_raider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "ca011fb1-fdc6-4eec-9344-c247339292ab"
fn saprazzan_raider() -> CardIndex {
    card_index("ca011fb1-fdc6-4eec-9344-c247339292ab")
}

/// `Saprazzan Raider` (`Coverage::Generated`):
/// "When this creature becomes blocked, return it to its owner's hand."
///
/// Verifies that when `Saprazzan Raider` attacks and becomes blocked, its
/// triggered ability returns it to its owner's hand before combat damage.
#[test]
fn saprazzan_raider_returns_to_owners_hand_when_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9609, island())
        .battlefield(0, &[saprazzan_raider()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let raider = on_battlefield(&engine, p0, saprazzan_raider()).expect("P0 Saprazzan Raider");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(raider, Defender::Player(p1))],
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
                blockers: vec![(bears, raider)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, saprazzan_raider()).is_none(),
        "Saprazzan Raider left the battlefield"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "Saprazzan Raider returned to its owner's hand"
    );
}
