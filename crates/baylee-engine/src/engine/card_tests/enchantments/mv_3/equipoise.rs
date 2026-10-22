//! `cards/enchantments/mv_3/equipoise.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn equipoise() -> CardIndex {
    card_index("aab905ef-b945-4523-b26f-fb5e6932bb56")
}

#[test]
fn equipoise_casts_and_enters_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[equipoise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, equipoise());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, equipoise()).is_some());
}
