//! `cards/enchantments/auras/mv_2/phantom_wings.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, PHANTOM_WINGS};

/// `Phantom Wings` ({1}{U}, Enchantment — Aura, Partial) grants flying to
/// enchanted creature and can be sacrificed as an activated ability cost.
#[test]
fn phantom_wings_grants_flying_and_can_be_sacrificed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[GRIZZLY_BEARS, island(), island()])
        .hand(0, &[PHANTOM_WINGS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, PHANTOM_WINGS);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::FLYING));

    activate(&mut engine, p0, PHANTOM_WINGS, 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, PHANTOM_WINGS).is_some());
    assert!(!keywords(&engine, bears).contains(KeywordSet::FLYING));
}
