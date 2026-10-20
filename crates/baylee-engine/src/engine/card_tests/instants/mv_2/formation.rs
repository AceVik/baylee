//! `cards/instants/mv_2/formation.rs`, played.
//!
//! Oracle:
//! - "Target creature gains banding until end of turn."
//! - "Draw a card at the beginning of the next turn's upkeep."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn formation() -> CardIndex {
    card_index("ad5d27ea-cead-4c95-8561-283855ce952e")
}

#[test]
fn formation_grants_banding_and_draws_at_next_turns_upkeep() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[formation()], &[]);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    lk::cast_at(&mut engine, P0, formation(), &[bears], &[]);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::BANDING)
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    lk::to_step(&mut engine, P0, Step::Upkeep);
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1
    );
}
