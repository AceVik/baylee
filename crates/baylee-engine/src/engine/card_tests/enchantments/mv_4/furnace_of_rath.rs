//! `cards/enchantments/mv_4/furnace_of_rath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn furnace_of_rath() -> CardIndex {
    card_index("68715465-6cf9-4006-87e9-31f227fe9ed3")
}

/// Furnace of Rath (`68715465-6cf9-4006-87e9-31f227fe9ed3`) — {1}{R}{R}{R} Enchantment (Partial):
/// "If a source would deal damage to a permanent or player, it deals double that damage to that permanent or player instead."
#[test]
fn furnace_of_rath_boosts_creature_power_on_the_battlefield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[furnace_of_rath(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    assert_eq!(pt(&engine, bears), (3, 2));
    assert_eq!(pt(&engine, giant), (4, 3));
}
