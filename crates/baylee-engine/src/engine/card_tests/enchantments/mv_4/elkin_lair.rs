//! `cards/enchantments/mv_4/elkin_lair.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn elkin_lair() -> CardIndex {
    card_index("b3488947-7142-4e6c-9c92-2728469b3ad9")
}

#[test]
fn elkin_lair_casts_and_enters_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[elkin_lair()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, elkin_lair());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, elkin_lair()).is_some());
}
