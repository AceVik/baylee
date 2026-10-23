//! `cards/enchantments/auras/mv_1/mob_mentality.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn mob_mentality() -> CardIndex {
    card_index("90e8fb2c-6473-42c3-92f4-f46a744920fa")
}

/// `Mob Mentality` ({R}, Enchantment — Aura, `Coverage::Partial`) enchants a
/// creature and grants it trample.
#[test]
fn mob_mentality_grants_trample_to_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), GRIZZLY_BEARS, llanowar_elves()])
        .hand(0, &[mob_mentality()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    assert!(!keywords(&engine, bear).contains(KeywordSet::TRAMPLE));
    cast_from_hand(&mut engine, p0, mob_mentality());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, mob_mentality()).is_some());
    assert!(keywords(&engine, bear).contains(KeywordSet::TRAMPLE));
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::TRAMPLE),
        "only the enchanted creature has trample"
    );
}
