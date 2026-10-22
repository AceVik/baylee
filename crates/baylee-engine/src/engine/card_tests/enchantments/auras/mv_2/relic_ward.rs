//! `cards/enchantments/auras/mv_2/relic_ward.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::JAYEMDAE_TOME;

fn relic_ward() -> CardIndex {
    card_index("c6321de2-80b0-4679-93cf-97df75344e97")
}

/// `Relic Ward` ({1}{W}, Enchantment — Aura, `Coverage::Partial`) has flash,
/// enchants an artifact, and grants it shroud.
#[test]
fn relic_ward_has_flash_and_grants_shroud_to_enchanted_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), JAYEMDAE_TOME])
        .hand(0, &[relic_ward()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tome = on_battlefield(&engine, p0, JAYEMDAE_TOME).unwrap();
    cast_from_hand(&mut engine, p0, relic_ward());
    aim_at(&mut engine, p0, tome);
    pass_until(&mut engine, stack_is_empty);

    let ward = on_battlefield(&engine, p0, relic_ward()).unwrap();
    assert!(keywords(&engine, ward).contains(KeywordSet::FLASH));
    assert!(keywords(&engine, tome).contains(KeywordSet::SHROUD));
}
