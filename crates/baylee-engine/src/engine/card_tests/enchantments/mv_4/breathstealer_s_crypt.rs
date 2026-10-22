//! `cards/enchantments/mv_4/breathstealer_s_crypt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn breathstealer_s_crypt() -> CardIndex {
    card_index("c9a9550c-e6bb-4fe3-930f-6266024dbead")
}

#[test]
fn breathstealers_crypt_casts_and_enters_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), swamp(), swamp()])
        .hand(0, &[breathstealer_s_crypt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, breathstealer_s_crypt());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, breathstealer_s_crypt()).is_some());
}
