//! `cards/enchantments/auras/mv_2/armor_of_thorns.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ARMOR_OF_THORNS, GRIZZLY_BEARS};

#[test]
fn armor_of_thorns_gives_enchanted_nonblack_creature_plus_two_plus_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[ARMOR_OF_THORNS])
        .battlefield(0, &[GRIZZLY_BEARS, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, ARMOR_OF_THORNS);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (4, 4));
}
