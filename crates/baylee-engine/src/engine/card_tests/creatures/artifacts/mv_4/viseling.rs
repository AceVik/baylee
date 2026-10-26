//! `cards/creatures/artifacts/mv_4/viseling.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn viseling_deals_hand_size_minus_four_damage_at_opponent_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[index::VISELING])
        .hand(1, &[island(); 6])
        .battlefield(0, &[island(); 4])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, index::VISELING);
    pass_until(&mut engine, stack_is_empty);

    let hand_count =
        i32::try_from(engine.state().zones.list(ZoneLocation::Hand(p1)).len()).unwrap();
    assert!(hand_count > 4);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 20 - (hand_count - 4));
}
