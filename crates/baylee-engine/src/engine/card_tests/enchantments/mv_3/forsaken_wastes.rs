//! `cards/enchantments/mv_3/forsaken_wastes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::FORSAKEN_WASTES;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Forsaken Wastes (0a282c88-3f02-4bf2-938b-5cd94dd7a24e) — {2}{B} World Enchantment:
/// "Players can't gain life. At the beginning of each player's upkeep, that
/// player loses 1 life."
#[test]
fn forsaken_wastes_makes_active_player_lose_one_life_at_upkeep() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[FORSAKEN_WASTES])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, FORSAKEN_WASTES);
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's main phase (passing through P1's upkeep trigger).
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(engine.state().players[0].life, 20);
}
