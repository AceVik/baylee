//! `cards/enchantments/auras/mv_2/binding_agony.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{
    BASALT_GOLEM, GRIZZLY_BEARS, HILL_GIANT, LIGHTNING_BOLT, PRODIGAL_SORCERER,
};

fn binding_agony() -> CardIndex {
    card_index("dcd41ed1-4a12-416b-8ba0-51a9c0927ddb")
}

#[test]
fn binding_agony_deals_damage_to_controller_when_enchanted_creature_is_damaged() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[binding_agony()])
        .battlefield(0, &[PRODIGAL_SORCERER, swamp(), swamp()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, binding_agony());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, PRODIGAL_SORCERER, 0);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
}

#[test]
fn binding_agony_ignores_damage_to_a_creature_it_does_not_enchant() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[binding_agony()])
        .battlefield(0, &[PRODIGAL_SORCERER, swamp(), swamp()])
        .battlefield(1, &[HILL_GIANT, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, binding_agony());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    // The ping hits the Bears, not the enchanted Giant: the Aura is silent.
    activate(&mut engine, p0, PRODIGAL_SORCERER, 0);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bear).unwrap().damage, 1);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().players[0].life, 20);
}

/// Binding Agony (Partial): "this Aura deals that much damage to that
/// creature's controller." The amount is not carried, so it deals 1. A pinned
/// limitation: this test moves when "that much" can be said.
#[test]
fn binding_agony_deals_only_one_for_a_bigger_hit() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[binding_agony(), LIGHTNING_BOLT])
        .battlefield(0, &[swamp(), swamp(), mountain(), mountain()])
        .battlefield(1, &[BASALT_GOLEM])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let golem = on_battlefield(&engine, p1, BASALT_GOLEM).unwrap();
    cast_from_hand(&mut engine, p0, binding_agony());
    aim_at(&mut engine, p0, golem);
    pass_until(&mut engine, stack_is_empty);
    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, golem);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(golem).unwrap().damage, 3);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "3 damage to the Golem costs its controller 1, not 3"
    );
}
