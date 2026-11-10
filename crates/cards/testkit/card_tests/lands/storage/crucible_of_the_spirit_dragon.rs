//! `cards/lands/storage/crucible_of_the_spirit_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn crucible_of_the_spirit_dragon_banks_storage_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[crucible_of_the_spirit_dragon(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let crucible = on_battlefield(&engine, p0, crucible_of_the_spirit_dragon())
        .expect("crucible on battlefield");
    assert_eq!(counters_on(&engine, crucible, counters::STORAGE), 0);

    // Tap Forest to float {G} without tapping Crucible itself.
    tap_all_mana_but(&mut engine, p0, Some(crucible_of_the_spirit_dragon()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(!is_tapped(&engine, crucible));

    activate(&mut engine, p0, crucible_of_the_spirit_dragon(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, crucible, counters::STORAGE), 1);
    assert!(is_tapped(&engine, crucible));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
