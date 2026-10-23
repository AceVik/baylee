//! `cards/instants/mv_2/repopulate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn repopulate() -> CardIndex {
    card_index("3a225d39-c1f8-4ab0-98ec-dad63bf268db")
}

#[test]
fn repopulate_cycles_for_two_and_discards_itself() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[], &[repopulate()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Colorless, 2)]);
    let before = engine.state().zones.list(ZoneLocation::Hand(lk::P0)).len();
    activate(&mut engine, lk::P0, repopulate(), 0);
    assert!(in_graveyard(&engine, lk::P0, repopulate()).is_some());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(lk::P0)).len(),
        before
    );
}
