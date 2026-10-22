//! `cards/enchantments/mv_3/desolation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn desolation() -> CardIndex {
    card_index("a80c3af8-470e-4e95-9579-cfdf18fbdbfa")
}

#[test]
fn desolation_casts_and_enters_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[desolation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, desolation());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, desolation()).is_some());
}
