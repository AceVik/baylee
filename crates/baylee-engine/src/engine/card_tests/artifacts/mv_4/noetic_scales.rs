#[allow(clippy::wildcard_imports)]
use super::*;

fn noetic_scales() -> CardIndex {
    card_index("aebddadd-f05b-4ad5-bda4-603a134f68dc")
}

#[test]
fn noetic_scales_enters_battlefield_as_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[noetic_scales()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, noetic_scales());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, noetic_scales()).is_some());
}
