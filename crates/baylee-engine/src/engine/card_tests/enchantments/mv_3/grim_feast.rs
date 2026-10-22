//! `cards/enchantments/mv_3/grim_feast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIM_FEAST, HILL_GIANT, LIGHTNING_BOLT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Grim Feast (a69c3fb0-80c6-480c-9ef6-b1d013a3414e) — {1}{B}{G} Enchantment:
/// "At the beginning of your upkeep, this enchantment deals 1 damage to you.
/// Whenever a creature is put into an opponent's graveyard from the
/// battlefield, you gain life equal to its toughness."
#[test]
fn grim_feast_gains_life_for_opponent_creature_dying() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), forest(), mountain(), mountain()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[GRIM_FEAST, LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, GRIM_FEAST);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Hill Giant (3/3) dies -> P0 gains 3 life (20 -> 23).
    assert_eq!(engine.state().players[0].life, 23);
}
