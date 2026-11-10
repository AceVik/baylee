//! `cards/enchantments/mv_4/burning_earth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn burning_earth() -> CardIndex {
    card_index("21b8192e-46a2-4a6a-abd5-3a074efa30a9")
}

/// "Whenever a player taps a nonbasic land for mana, this enchantment deals
/// 1 damage to that player": whoever taps a Badlands takes 1, whoever taps a
/// basic Forest takes nothing.
#[test]
fn burning_earth_hurts_whoever_taps_a_nonbasic_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[burning_earth(), badlands(), forest()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forest_id = on_battlefield(&engine, p0, forest()).expect("seated");
    tap_mana_where(&mut engine, p0, |id| id == forest_id);
    assert!(stack_is_empty(&engine), "a basic land triggers nothing");

    let badlands_id = on_battlefield(&engine, p0, badlands()).expect("seated");
    tap_mana_where(&mut engine, p0, |id| id == badlands_id);
    assert!(!stack_is_empty(&engine), "the damage waits on the stack");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 19, "I tapped it, I take 1");
    assert_eq!(life_of(&engine, p1), 20);

    reach_their_main_phase(&mut engine, p1);
    let theirs = on_battlefield(&engine, p1, badlands()).expect("seated");
    tap_mana_where(&mut engine, p1, |id| id == theirs);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 19, "the opponent takes it too");
    assert_eq!(life_of(&engine, p0), 19, "and I take nothing for it");
}
