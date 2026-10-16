//! `cards/creatures/artifacts/mv_3/moriok_replica.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn moriok_replica() -> CardIndex {
    card_index("6273cd72-5330-4712-ae73-b058023f0435")
}

/// Moriok Replica — "{1}{B}, Sacrifice this creature: You draw two cards and
/// you lose 2 life."
#[test]
fn moriok_replica_sacrifices_to_draw_two_and_lose_two() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[moriok_replica(), swamp(), forest()]);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_of(&engine, p0).len();
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, moriok_replica(), 0);
    assert!(
        on_battlefield(&engine, p0, moriok_replica()).is_none(),
        "sacrificed as a cost"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, moriok_replica()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );
    assert_eq!(library_of(&engine, p0).len(), library_before - 2);
    assert_eq!(life_of(&engine, 0), 18);
    assert_eq!(life_of(&engine, 1), 20);
}
