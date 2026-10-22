//! `cards/enchantments/auras/mv_1/spider_climb.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn spider_climb() -> CardIndex {
    card_index("2636aafa-83f9-4b72-b729-04e1c3aa0eec")
}

/// `Spider Climb` ({G}, Enchantment — Aura, `Coverage::Partial`) has flash,
/// enchants a creature, and grants it +0/+3 and reach.
#[test]
fn spider_climb_grants_plus_0_plus_3_and_reach_to_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), GRIZZLY_BEARS, HILL_GIANT])
        .hand(0, &[spider_climb()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let other = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, spider_climb());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (2, 5));
    assert!(keywords(&engine, bear).contains(KeywordSet::REACH));
    assert_eq!(pt(&engine, other), (3, 3));
}
