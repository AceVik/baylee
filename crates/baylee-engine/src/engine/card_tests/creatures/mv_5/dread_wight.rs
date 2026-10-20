//! `cards/creatures/mv_5/dread_wight.rs`, played.
//!
//! Oracle:
//! - "At end of combat, put a paralyzation counter on each creature blocking or blocked by this creature and tap those creatures. Each of those creatures doesn't untap during its controller's untap step for as long as it has a paralyzation counter on it. Each of those creatures gains '{4}: Remove a paralyzation counter from this creature.'"

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "f9078d79-3250-44e8-aba1-ab52e0ba4425"
fn dread_wight() -> CardIndex {
    card_index("f9078d79-3250-44e8-aba1-ab52e0ba4425")
}

#[test]
fn dread_wight_puts_paralyzation_counter_and_taps_blocking_creature_at_end_of_combat() {
    let mut engine = lk::game(SEED, &[dread_wight()], &[index::WALL_OF_STONE], &[], &[]);
    let wight = lk::obj(&engine, P0, dread_wight());
    let wall = lk::obj(&engine, P1, index::WALL_OF_STONE);

    let _ = lk::attack(&mut engine, &[wight]);
    lk::block(&mut engine, &[(wall, wight)]);

    // Advance past end of combat: Wall of Stone gets a paralyzation counter and becomes tapped.
    lk::through_combat(&mut engine);

    let wall_obj = engine.state().object(wall).unwrap();
    assert_eq!(
        wall_obj.counters.get(counters::PARALYZATION),
        1,
        "at end of combat, put a paralyzation counter on each creature blocking Dread Wight"
    );
    assert!(
        wall_obj.status.contains(Status::TAPPED),
        "and tap those creatures"
    );
}
