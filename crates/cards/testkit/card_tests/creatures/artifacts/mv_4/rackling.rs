//! `cards/artifacts/mv_4/rackling.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn rackling_deals_three_minus_hand_size_damage_at_opponent_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[card_index("41d116d7-3b1e-4777-b3c0-54f8ca6b2785")])
        .hand(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    // At p1's upkeep, p1 had 1 card in hand, so Rackling dealt 3 - 1 = 2 damage.
    assert_eq!(engine.state().players[1].life, 18);
}

#[test]
fn rackling_deals_no_damage_when_opponent_has_three_or_more_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[card_index("41d116d7-3b1e-4777-b3c0-54f8ca6b2785")])
        .hand(1, &[swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 20);
}
