//! `cards/enchantments/mv_2/parapet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn parapet() -> CardIndex {
    card_index("4e73634a-9df8-4c36-b1d2-f366eae902fc")
}

/// `Parapet` ({1}{W}, Enchantment, `Coverage::Partial`) has flash and gives
/// creatures you control +0/+1.
#[test]
fn parapet_has_flash_and_gives_your_creatures_plus_0_plus_1() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), GRIZZLY_BEARS])
        .hand(0, &[parapet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&engine, bear), (2, 2));

    cast_from_hand(&mut engine, p0, parapet());
    pass_until(&mut engine, stack_is_empty);

    let ench = on_battlefield(&engine, p0, parapet()).unwrap();
    assert!(keywords(&engine, ench).contains(KeywordSet::FLASH));
    assert_eq!(pt(&engine, bear), (2, 3));
}
