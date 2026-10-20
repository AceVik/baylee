//! `cards/instants/mv_1/heal.rs`, played.
//!
//! Oracle:
//! - "Prevent the next 1 damage that would be dealt to any target this turn."
//! - "Draw a card at the beginning of the next turn's upkeep."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn heal() -> CardIndex {
    card_index("013c42a4-d555-46c9-9d57-b50d6b59ae9e")
}

#[test]
fn heal_prevents_one_damage_and_draws_next_upkeep() {
    let mut engine = lk::game(SEED, &[], &[], &[heal(), index::LIGHTNING_BOLT], &[]);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::White, 1), (ManaColor::Red, 1)],
    );
    lk::cast_at(&mut engine, P0, heal(), &[], &[P0]);
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[], &[P0]);
    assert_eq!(lk::life(&engine, P0), 18);

    assert!(in_hand(&engine, P0, index::FOREST).is_none());
    lk::to_step(&mut engine, P1, Step::Upkeep);
    lk::settle(&mut engine);
    assert!(in_hand(&engine, P0, index::FOREST).is_some());
}
