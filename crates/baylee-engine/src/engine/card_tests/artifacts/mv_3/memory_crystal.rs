//! `cards/artifacts/mv_3/memory_crystal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn memory_crystal() -> CardIndex {
    card_index("f5fd3141-6fd9-4491-9f29-7fd81209bd57")
}

#[test]
fn memory_crystal_resolves_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[memory_crystal()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, memory_crystal());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, memory_crystal()).is_some());
}
