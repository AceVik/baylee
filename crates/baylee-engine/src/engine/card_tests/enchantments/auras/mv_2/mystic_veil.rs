//! `cards/enchantments/auras/mv_2/mystic_veil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn mystic_veil() -> CardIndex {
    card_index("43c88fcd-aac9-4741-8fd6-31ac81c5bb6f")
}

/// `Mystic Veil` ({1}{U}, Enchantment — Aura, `Coverage::Partial`) has flash,
/// enchants a creature, and grants it shroud.
#[test]
fn mystic_veil_has_flash_and_grants_shroud_to_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), GRIZZLY_BEARS])
        .hand(0, &[mystic_veil()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, mystic_veil());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    let veil = on_battlefield(&engine, p0, mystic_veil()).unwrap();
    assert!(keywords(&engine, veil).contains(KeywordSet::FLASH));
    assert!(keywords(&engine, bear).contains(KeywordSet::SHROUD));
}
