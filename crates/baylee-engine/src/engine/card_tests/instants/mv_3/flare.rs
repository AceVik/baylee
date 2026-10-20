//! `cards/instants/mv_3/flare.rs`, played.
//!
//! Oracle:
//! - "Flare deals 1 damage to any target."
//! - "Draw a card at the beginning of the next turn's upkeep."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flare() -> CardIndex {
    card_index("ff8fae94-3a34-4bc6-aaa2-7ab7d2e2d0dd")
}

#[test]
fn flare_deals_one_damage_and_draws_at_next_turns_upkeep() {
    let mut engine = lk::game(SEED, &[], &[], &[flare()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
    );
    lk::cast_at(&mut engine, P0, flare(), &[], &[P1]);
    assert_eq!(lk::life(&engine, P1), 19);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    lk::to_step(&mut engine, P0, Step::Upkeep);
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1
    );
}
