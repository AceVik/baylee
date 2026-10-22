//! `cards/enchantments/mv_3/city_of_solitude.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn city_of_solitude() -> CardIndex {
    card_index("671458ac-7441-4332-9e59-a42ec8b62f31")
}

#[test]
fn city_of_solitude_casts_and_enters_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[city_of_solitude()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, city_of_solitude());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, city_of_solitude()).is_some());
}
