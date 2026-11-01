//! `cards/creatures/mv_2/pyric_salamander.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "745a2815-8487-4b78-93b7-d1d4ef6adc8f"
fn pyric_salamander() -> CardIndex {
    card_index("745a2815-8487-4b78-93b7-d1d4ef6adc8f")
}

#[test]
fn pyric_salamander_pumps_and_sacrifices_at_end_step() {
    let mut engine = lk::game(SEED, &[pyric_salamander()], &[], &[], &[]);
    let salamander = lk::obj(&engine, P0, pyric_salamander());

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::use_ability(&mut engine, P0, pyric_salamander(), 0, &[], &[]);

    let ch = engine.state().object(salamander).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(2), Some(1)));
    assert!(
        on_battlefield(&engine, P0, pyric_salamander()).is_some(),
        "the sacrifice waits for the next end step"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(on_battlefield(&engine, P0, pyric_salamander()).is_none());
    assert!(in_graveyard(&engine, P0, pyric_salamander()).is_some());
}

/// The sacrifice belongs to the pump: a Salamander nobody pumped is still
/// there when the end step has come and gone.
#[test]
fn pyric_salamander_that_was_not_pumped_is_not_sacrificed() {
    let mut engine = lk::game(SEED, &[pyric_salamander()], &[], &[], &[]);

    reach_their_main_phase(&mut engine, P1);
    assert!(on_battlefield(&engine, P0, pyric_salamander()).is_some());
}
