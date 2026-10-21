//! `cards/creatures/mv_7/chaos_lord.rs`, played.
//!
//! Oracle:
//! - "First strike"
//! - "At the beginning of your upkeep, target opponent gains control of this creature if the number of permanents is even." (not played: the card is Partial)
//! - "This creature can attack as though it had haste unless it entered this turn." (not played: the card is Partial)

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "16d0aea2-4ca4-4a9d-8674-2863e085f514"
fn chaos_lord() -> CardIndex {
    card_index("16d0aea2-4ca4-4a9d-8674-2863e085f514")
}

/// First strike played through combat: a 6/4 Craw Wurm blocking Chaos Lord
/// dies in the first-strike damage step and never deals its 6. Without first
/// strike both would deal damage at once and the Lord would end combat with
/// 6 damage marked.
#[test]
fn chaos_lord_kills_its_blocker_before_the_blocker_deals_damage() {
    let mut engine = lk::game(SEED, &[chaos_lord()], &[index::CRAW_WURM], &[], &[]);
    let lord = lk::obj(&engine, P0, chaos_lord());
    let wurm = lk::obj(&engine, P1, index::CRAW_WURM);

    let _ = lk::attack(&mut engine, &[lord]);
    lk::block(&mut engine, &[(wurm, lord)]);
    lk::through_combat(&mut engine);

    assert!(
        on_battlefield(&engine, P1, index::CRAW_WURM).is_none(),
        "the blocker died to Chaos Lord's 7 damage"
    );
    assert_eq!(
        lk::damage_on(&engine, lord),
        0,
        "first strike: the Wurm was dead before regular combat damage"
    );
}
