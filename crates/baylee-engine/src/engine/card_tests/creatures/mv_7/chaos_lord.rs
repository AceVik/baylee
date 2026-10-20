//! `cards/creatures/mv_7/chaos_lord.rs`, played.
//!
//! Oracle:
//! - "First strike"
//! - "At the beginning of your upkeep, target opponent gains control of this creature if the number of permanents is even."
//! - "This creature can attack as though it had haste unless it entered this turn."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "16d0aea2-4ca4-4a9d-8674-2863e085f514"
fn chaos_lord() -> CardIndex {
    card_index("16d0aea2-4ca4-4a9d-8674-2863e085f514")
}

#[test]
fn chaos_lord_is_a_seven_seven_with_first_strike() {
    let mut engine = lk::game(SEED, &[], &[], &[chaos_lord()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 4), (ManaColor::Red, 3)],
    );
    cast_with_floating(&mut engine, P0, chaos_lord());
    pass_until(&mut engine, stack_is_empty);

    let lord = on_battlefield(&engine, P0, chaos_lord()).expect("Chaos Lord on battlefield");
    assert_eq!(pt(&engine, lord), (7, 7));
    assert!(
        engine
            .state()
            .object(lord)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE),
        "Chaos Lord has first strike"
    );
}
