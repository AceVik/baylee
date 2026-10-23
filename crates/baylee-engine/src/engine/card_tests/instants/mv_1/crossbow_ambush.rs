//! `cards/instants/mv_1/crossbow_ambush.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn crossbow_ambush() -> CardIndex {
    card_index("c10fd7a6-360f-43d6-a4ef-d990c97cb2f6")
}

/// Crossbow Ambush (`c10fd7a6-360f-43d6-a4ef-d990c97cb2f6`) — {G} Instant (Implemented):
/// "Creatures you control gain reach until end of turn."
#[test]
fn crossbow_ambush_grants_reach_to_creatures_you_control() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[crossbow_ambush()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    assert!(!keywords(&engine, bears).contains(KeywordSet::REACH));

    cast_from_hand(&mut engine, P0, crossbow_ambush());
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::REACH));
    let enemy = on_battlefield(&engine, lk::P1, index::GRIZZLY_BEARS).unwrap();
    assert!(!keywords(&engine, enemy).contains(KeywordSet::REACH));
    reach_their_main_phase(&mut engine, lk::P1);
    assert!(!keywords(&engine, bears).contains(KeywordSet::REACH));
}
