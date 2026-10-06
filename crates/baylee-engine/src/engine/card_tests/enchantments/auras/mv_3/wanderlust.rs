//! `cards/enchantments/auras/mv_3/wanderlust.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wanderlust: "Enchant creature" / "At the beginning of the upkeep of
/// enchanted creature's controller, this Aura deals 1 damage to that
/// player."
#[test]
fn wanderlust_deals_upkeep_damage_to_the_enchanted_creatures_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let wanderlust = wanderlust();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[wanderlust])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p1, quiet_creature()).expect("their creature");

    cast_from_hand(&mut engine, p0, wanderlust);
    aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "1 damage at its controller's upkeep"
    );
    assert_eq!(engine.state().players[0].life, 20);

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "not at Wanderlust's own controller's upkeep, only the enchanted creature's"
    );
    assert_eq!(engine.state().players[1].life, 19);
}
