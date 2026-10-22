//! `cards/enchantments/mv_5/death_pits_of_rath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn death_pits_of_rath() -> CardIndex {
    card_index("aead53ef-ffcf-4ae2-ab81-40aa9627adc9")
}

/// Death Pits of Rath (`aead53ef-ffcf-4ae2-ab81-40aa9627adc9`) — {3}{B}{B} Enchantment (Partial):
/// "Whenever a creature is dealt damage, destroy it. It can't be regenerated."
#[test]
fn death_pits_of_rath_destroys_a_creature_dealt_nonlethal_damage() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[death_pits_of_rath(), index::PRODIGAL_SORCERER])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    activate(&mut engine, P0, index::PRODIGAL_SORCERER, 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, P1, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, P0, index::PRODIGAL_SORCERER).is_some());
}
