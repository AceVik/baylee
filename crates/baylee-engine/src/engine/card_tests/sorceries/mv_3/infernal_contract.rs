//! `cards/sorceries/mv_3/infernal_contract.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::INFERNAL_CONTRACT;

const P0: PlayerId = PlayerId::new(0);

/// Infernal Contract (ae40db20-9283-4209-81b1-712119272b83) — {B}{B}{B} Sorcery:
/// "Draw four cards. You lose half your life, rounded up."
#[test]
fn infernal_contract_draws_four_cards_and_loses_half_life_rounded_up() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[INFERNAL_CONTRACT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let before_hand = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, INFERNAL_CONTRACT);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        before_hand - 1 + 4
    );
    assert_eq!(engine.state().players[0].life, 10);
}
