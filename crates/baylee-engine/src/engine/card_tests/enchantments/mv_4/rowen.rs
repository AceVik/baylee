//! `cards/enchantments/mv_4/rowen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rowen() -> CardIndex {
    card_index("eb44f5eb-1c6a-49f0-9751-23d167bf8288")
}

/// `Rowen` ({2}{G}{G}, Enchantment, `Coverage::Partial`) enters the battlefield
/// as an enchantment.
#[test]
fn rowen_enters_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[rowen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rowen());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, rowen()).is_some());
}
