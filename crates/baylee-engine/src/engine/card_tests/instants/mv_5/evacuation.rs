//! `cards/instants/mv_5/evacuation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn evacuation() -> CardIndex {
    card_index("fdd94383-b573-439a-8e1c-925af887c5a6")
}

/// Evacuation (`fdd94383-b573-439a-8e1c-925af887c5a6`) — {3}{U}{U} Instant (Implemented):
/// "Return all creatures to their owners' hands."
#[test]
fn evacuation_returns_all_creatures_to_owners_hands() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT, index::SOL_RING])
        .hand(0, &[evacuation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, evacuation());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, P1, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, P1, index::SOL_RING).is_some());
    assert_eq!(all_on_battlefield(&engine, P0, island()).len(), 5);
}
