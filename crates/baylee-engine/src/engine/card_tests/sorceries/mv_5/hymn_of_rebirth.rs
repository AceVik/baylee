//! `cards/sorceries/mv_5/hymn_of_rebirth.rs`, played.
//!
//! Oracle:
//! - "Put target creature card from a graveyard onto the battlefield under your control."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hymn_of_rebirth() -> CardIndex {
    card_index("f06fca68-593b-4fbd-a5a9-618060311172")
}

#[test]
fn hymn_of_rebirth_reanimates_creature_from_opponent_graveyard_under_your_control() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[hymn_of_rebirth()],
        &[index::GRIZZLY_BEARS],
    );
    let dead_bears = hand_to_graveyard(&mut engine, P1, index::GRIZZLY_BEARS);
    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 3),
            (ManaColor::Green, 1),
            (ManaColor::White, 1),
        ],
    );
    lk::cast_at(&mut engine, P0, hymn_of_rebirth(), &[dead_bears], &[]);
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
