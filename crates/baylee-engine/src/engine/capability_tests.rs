//! What a seat is allowed to do beyond answering its own choices.
//!
//! There used to be a `GamePreset::dev_mode` flag. Nothing read it, and it
//! arrived over the wire in `CreateGame` — so the one thing it did do was
//! let whoever opened the socket ask to be trusted. Capabilities replaced
//! it: per seat, granted by the host, never inbound, and empty by default.

use super::testkit::{Duel, card_index};
use super::*;
use baylee_core::ids::CardIndex;
use baylee_core::preset::SeatCapabilities;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// A seat with no `dev_commands` capability cannot reach the state, and in a
/// lobby game that is every seat. The old `state_mut_dev()` asked nobody:
/// anything holding an `&mut Engine` could rewrite the board, which is one
/// careless admin endpoint away from a ranked game being editable.
#[test]
fn only_a_seat_granted_dev_commands_can_reach_the_state() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(31, forest()).start();
    assert!(
        engine.dev_state_mut(p0).is_some(),
        "the test harness grants itself dev commands"
    );

    let mut plain = Duel::new(31, forest()).without_capabilities().start();
    assert!(
        plain.dev_state_mut(p0).is_none(),
        "a seat with no capability was handed the game state"
    );
    assert!(
        plain.dev_state_mut(PlayerId::new(9)).is_none(),
        "a seat that does not exist got an answer at all"
    );
    assert_eq!(plain.capabilities(p0), SeatCapabilities::default());
}
