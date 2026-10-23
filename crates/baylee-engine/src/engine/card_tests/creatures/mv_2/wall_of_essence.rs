//! `cards/creatures/mv_2/wall_of_essence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn wall_of_essence() -> CardIndex {
    card_index("1374ac18-01bd-452f-af72-a5f96285ed69")
}

/// Wall of Essence (`1374ac18-01bd-452f-af72-a5f96285ed69`) — {1}{W} Creature — Wall (Partial):
/// "Defender. Whenever this creature is dealt combat damage, you gain that much life."
#[test]
fn wall_of_essence_gains_life_when_dealt_damage() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[wall_of_essence(), mountain()])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let wall = on_battlefield(&engine, P0, wall_of_essence()).unwrap();
    cast_from_hand(&mut engine, P0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, P0, wall);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 21);
}
