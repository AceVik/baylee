//! `cards/creatures/artifacts/mv_3/sarcomite_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sarcomite_myr() -> CardIndex {
    card_index("e78fd8f6-eca9-4107-aee7-a65689cee2f4")
}

/// Sarcomite Myr — "{2}: This creature gains flying until end of turn."
#[test]
fn sarcomite_myr_gains_flying_for_two() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[sarcomite_myr(), forest(), forest()]);
    let myr = on_battlefield(&engine, p0, sarcomite_myr()).expect("seated");
    assert_eq!(pt(&engine, myr), (2, 1));
    assert!(!keywords(&engine, myr).contains(KeywordSet::FLYING));
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, sarcomite_myr(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, myr).contains(KeywordSet::FLYING));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// "{2}, Sacrifice this creature: Draw a card."
#[test]
fn sarcomite_myr_sacrifices_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[sarcomite_myr(), forest(), forest()]);
    let library_before = library_of(&engine, p0).len();
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, sarcomite_myr(), 1);
    assert!(on_battlefield(&engine, p0, sarcomite_myr()).is_none());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, sarcomite_myr()).is_some());
    assert_eq!(library_of(&engine, p0).len(), library_before - 1);
}
