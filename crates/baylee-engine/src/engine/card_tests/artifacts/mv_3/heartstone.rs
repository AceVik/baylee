//! `cards/artifacts/mv_3/heartstone.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn heartstone() -> CardIndex {
    card_index("8f8ebf49-37c5-4da0-9cee-84ad22ab5d37")
}

fn mindwarper() -> CardIndex {
    card_index("efb42ccc-6fed-47f0-84b4-e2a9a42e4dfa")
}

/// Heartstone (`8f8ebf49-37c5-4da0-9cee-84ad22ab5d37`) — {3} Artifact (Implemented):
/// "Activated abilities of creatures cost {1} less to activate. This effect can't reduce the mana in that cost to less than one mana."
#[test]
fn heartstone_reduces_generic_activated_ability_cost_to_minimum_one() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[heartstone(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[mindwarper()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Cast Mindwarper ({2}{B}{B}) using the 4 Swamps; then untap 2 Swamps so P0 has {B}{B}
    // available, which pays Mindwarper's {2}{B} ability reduced by {1} from Heartstone.
    cast_from_hand(&mut engine, P0, mindwarper());
    pass_until(&mut engine, stack_is_empty);

    for id in all_on_battlefield(&engine, P0, swamp()).into_iter().take(2) {
        engine
            .dev_state_mut(P0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .remove(Status::TAPPED);
    }
    engine.refresh_offer();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, mindwarper(), 0);
}
