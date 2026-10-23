//! `cards/artifacts/mv_3/ensnaring_bridge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn ensnaring_bridge() -> CardIndex {
    card_index("dade5835-ccaf-45e3-b9a3-46017e0384b3")
}

/// Ensnaring Bridge (`dade5835-ccaf-45e3-b9a3-46017e0384b3`) — {3} Artifact (Partial):
/// "Creatures with power greater than the number of cards in your hand can't attack."
/// In Partial mode, prevents creatures with power 1 or greater from attacking while your hand is empty.
#[test]
fn ensnaring_bridge_stops_creatures_from_attacking_when_hand_is_empty() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), index::GRIZZLY_BEARS])
        .hand(0, &[ensnaring_bridge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    assert!(!keywords(&engine, bears).contains(KeywordSet::CANT_ATTACK));

    cast_from_hand(&mut engine, P0, ensnaring_bridge());
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::CANT_ATTACK));
}
