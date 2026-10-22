//! `cards/enchantments/mv_4/eye_of_singularity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn eye_of_singularity() -> CardIndex {
    card_index("e13edf92-cdba-4c24-b81b-088b1554fde6")
}

#[test]
fn eye_of_singularity_casts_and_enters_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[eye_of_singularity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, eye_of_singularity());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, eye_of_singularity()).is_some());
}
