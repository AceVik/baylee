//! `cards/enchantments/auras/mv_2/binding_agony.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BINDING_AGONY, HILL_GIANT, PRODIGAL_SORCERER};

#[test]
fn binding_agony_deals_damage_to_controller_when_enchanted_creature_is_damaged() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[BINDING_AGONY])
        .battlefield(0, &[PRODIGAL_SORCERER, swamp(), swamp()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, BINDING_AGONY);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, PRODIGAL_SORCERER, 0);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
}
