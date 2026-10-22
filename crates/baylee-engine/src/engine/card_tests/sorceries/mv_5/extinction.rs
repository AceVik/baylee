//! `cards/sorceries/mv_5/extinction.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn extinction() -> CardIndex {
    card_index("f3f01fe8-0a2c-4afa-94d2-268d27c0d252")
}

/// Extinction (`f3f01fe8-0a2c-4afa-94d2-268d27c0d252`) — {4}{B} Sorcery (Partial):
/// "Destroy all creatures of the creature type of your choice."
#[test]
fn extinction_destroys_creatures_on_the_battlefield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .hand(0, &[extinction()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, extinction());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, P1, index::HILL_GIANT).is_some());
}
