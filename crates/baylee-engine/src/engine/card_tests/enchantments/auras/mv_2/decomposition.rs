//! `cards/enchantments/auras/mv_2/decomposition.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BOG_IMP, DECOMPOSITION, LIGHTNING_BOLT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Decomposition (62817ef5-9b8d-4bca-bdd4-ab3bc4e063d0) — {1}{G} Aura:
/// "Enchant black creature. Enchanted creature has 'Cumulative upkeep—Pay 1
/// life.' When enchanted creature dies, its controller loses 2 life."
#[test]
fn decomposition_triggers_life_loss_for_controller_when_enchanted_black_creature_dies() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), mountain(), mountain()])
        .battlefield(1, &[BOG_IMP])
        .hand(0, &[DECOMPOSITION, LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let imp = on_battlefield(&engine, P1, BOG_IMP).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, DECOMPOSITION);
    aim_at(&mut engine, P0, imp);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![imp],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, BOG_IMP).is_none());
    assert_eq!(engine.state().players[1].life, 18);
}
