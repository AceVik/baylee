//! `cards/instants/mv_1/mind_games.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mind_games() -> CardIndex {
    card_index("ea43372f-b0a7-4d1b-be3e-a4a738b06d59")
}

/// Mind Games (`ea43372f-b0a7-4d1b-be3e-a4a738b06d59`) — {U} Instant (Partial):
/// "Tap target artifact, creature, or land."
#[test]
fn mind_games_taps_target_artifact_creature_or_land() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[mind_games()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bear = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, mind_games());
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, bear));
}
