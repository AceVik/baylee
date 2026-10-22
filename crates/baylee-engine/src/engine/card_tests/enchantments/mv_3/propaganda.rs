//! `cards/enchantments/mv_3/propaganda.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn propaganda() -> CardIndex {
    card_index("ea9709b6-4c37-4d5a-b04d-cd4c42e4f9dd")
}

/// `Propaganda` ({2}{U}, Enchantment, `Coverage::Partial`):
/// enters the battlefield; taxing attacking creatures `{2}` per attacker is not
/// in `Modifier`.
#[test]
fn propaganda_casts_and_enters_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[propaganda()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, propaganda());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, propaganda()).is_some());
}
