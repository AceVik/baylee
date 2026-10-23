//! `cards/creatures/mv_2/wall_of_souls.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wall_of_souls() -> CardIndex {
    card_index("2c85c9ea-c3fd-4981-a65c-4405bd87764e")
}

/// Wall of Souls (`2c85c9ea-c3fd-4981-a65c-4405bd87764e`) — {1}{B} Creature — Wall (Partial):
/// "Defender. Whenever this creature is dealt combat damage, it deals that much damage to target opponent or planeswalker."
#[test]
fn wall_of_souls_deals_damage_to_target_opponent_when_dealt_damage() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[wall_of_souls(), mountain()])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let wall = on_battlefield(&engine, P0, wall_of_souls()).unwrap();
    cast_from_hand(&mut engine, P0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, P0, wall);
    kit::settle(&mut engine, P0, |_, p| kit::choose_targets(p, &[], &[P1]));

    assert_eq!(engine.state().players[1].life, 19);
}
