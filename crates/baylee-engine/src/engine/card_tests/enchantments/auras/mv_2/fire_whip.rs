//! `cards/enchantments/auras/mv_2/fire_whip.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{FIRE_WHIP, GRIZZLY_BEARS};

/// `Fire Whip` ({1}{R}, Enchantment — Aura) grants enchanted creature a tap
/// ability to deal 1 damage to any target, and can be sacrificed to deal 1
/// damage to any target.
#[test]
fn fire_whip_grants_ping_to_enchanted_creature_and_sacrifices_for_1_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[GRIZZLY_BEARS, mountain(), mountain()])
        .hand(0, &[FIRE_WHIP])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, FIRE_WHIP);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    activate(
        &mut engine,
        p0,
        GRIZZLY_BEARS,
        crate::choice::GRANTED_ABILITY,
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19);

    activate(&mut engine, p0, FIRE_WHIP, 2);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
    assert!(in_graveyard(&engine, p0, FIRE_WHIP).is_some());
}
