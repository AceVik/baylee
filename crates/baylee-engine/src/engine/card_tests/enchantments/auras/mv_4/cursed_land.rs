//! `cards/enchantments/auras/mv_4/cursed_land.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cursed Land: "Enchant land" / "At the beginning of the upkeep of
/// enchanted land's controller, this Aura deals 1 damage to that player."
#[test]
fn cursed_land_deals_upkeep_damage_to_the_enchanted_lands_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let cursed_land = cursed_land();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[forest()])
        .hand(0, &[cursed_land])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p1, forest()).expect("their land");

    cast_from_hand(&mut engine, p0, cursed_land);
    aim_at(&mut engine, p0, land);
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
        "not at Cursed Land's own controller's upkeep, only the enchanted land's"
    );
    assert_eq!(engine.state().players[1].life, 19);
}
