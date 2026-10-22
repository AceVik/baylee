//! `cards/enchantments/mv_4/aether_flash.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn aether_flash() -> CardIndex {
    card_index("a3c35742-e306-49b6-b042-db4f685c6f86")
}

/// `Aether Flash` ({2}{R}{R}, Enchantment) deals 2 damage to each creature as
/// it enters the battlefield.
#[test]
fn aether_flash_deals_two_damage_to_entering_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[aether_flash(), GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, aether_flash());
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
}
