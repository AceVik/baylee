//! `cards/enchantments/auras/mv_1/mortal_wound.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HILL_GIANT, PRODIGAL_SORCERER};

fn mortal_wound() -> CardIndex {
    card_index("f7b70077-5518-45f9-9833-01a4f8afac71")
}

/// `Mortal Wound` ({G}, Enchantment — Aura, `Coverage::Implemented`):
/// `Enchant creature. When enchanted creature is dealt damage, destroy it.`
#[test]
fn mortal_wound_destroys_enchanted_creature_when_dealt_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), PRODIGAL_SORCERER])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[mortal_wound()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, PlayerId::new(1), HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, p0, mortal_wound());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    // Prodigal Sorcerer deals 1 non-lethal damage to the 3/3 Hill Giant; Mortal Wound destroys it.
    activate(&mut engine, p0, PRODIGAL_SORCERER, 0);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().zone, Zone::Graveyard);
}

/// "When enchanted creature is dealt damage": damage to another creature
/// destroys nothing, and the enchanted one, undamaged, stays.
#[test]
fn mortal_wound_ignores_damage_dealt_to_another_creature() {
    use baylee_core::generated::index::GRIZZLY_BEARS;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), PRODIGAL_SORCERER])
        .battlefield(1, &[HILL_GIANT, GRIZZLY_BEARS])
        .hand(0, &[mortal_wound()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, mortal_wound());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    // The Sorcerer pings the unenchanted Bears: 1 damage, not lethal to a 2/2.
    activate(&mut engine, p0, PRODIGAL_SORCERER, 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bears).unwrap().damage, 1);
    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(giant).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(giant).unwrap().damage, 0);
}
