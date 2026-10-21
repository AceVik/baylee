//! `cards/sorceries/mv_3/pox.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn pox() -> CardIndex {
    card_index("539d7c1a-b5e9-4204-9109-ddb2e8dec168")
}

/// Pox (539d7c1a-b5e9-4204-9109-ddb2e8dec168) — {B}{B}{B} Sorcery:
/// Coverage::Partial — resolves with no effect.
#[test]
fn pox_casts_for_three_black_and_resolves_to_the_graveyard() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[pox()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, pox());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, pox()).is_some());
}
