//! `cards/instants/mv_2/enervate.rs`, played.
//!
//! Oracle:
//! - "Tap target artifact, creature, or land."
//! - "Draw a card at the beginning of the next turn's upkeep."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn enervate() -> CardIndex {
    card_index("f763d95e-87af-473f-9b1f-cf16221e5792")
}

#[test]
fn enervate_taps_target_and_draws_at_next_turns_upkeep() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[enervate()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    lk::cast_at(&mut engine, P0, enervate(), &[bears], &[]);
    assert!(lk::tapped(&engine, bears));

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    lk::to_step(&mut engine, P0, Step::Upkeep);
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1
    );
}
