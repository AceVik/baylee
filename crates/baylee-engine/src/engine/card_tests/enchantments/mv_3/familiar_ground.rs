//! `cards/enchantments/mv_3/familiar_ground.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::FAMILIAR_GROUND;

/// `Familiar Ground` ({2}{G}, Enchantment) enters the battlefield.
#[test]
fn familiar_ground_enters_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[FAMILIAR_GROUND])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, FAMILIAR_GROUND);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, FAMILIAR_GROUND).is_some());
}
