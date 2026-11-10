//! `cards/enchantments/auras/mv_2/fractured_loyalty.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HILL_GIANT, PRODIGAL_SORCERER};

fn fractured_loyalty() -> CardIndex {
    card_index("ccc168ef-b93b-4bff-89a3-80984d3625de")
}

/// Fractured Loyalty — {1}{R} Enchantment — Aura
/// "Enchant creature"
/// "Whenever enchanted creature becomes the target of a spell or ability, that spell or ability's controller gains control of that creature."
#[test]
fn fractured_loyalty_gives_control_to_player_who_targets_enchanted_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[fractured_loyalty()])
        .battlefield(0, &[mountain(), mountain(), PRODIGAL_SORCERER])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, fractured_loyalty());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().controller, p1);

    // P0 targets the enchanted Hill Giant with Prodigal Sorcerer's activated ability.
    activate(&mut engine, p0, PRODIGAL_SORCERER, 0);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().controller, p0);
    assert_eq!(engine.state().object(giant).unwrap().damage, 1);
}
