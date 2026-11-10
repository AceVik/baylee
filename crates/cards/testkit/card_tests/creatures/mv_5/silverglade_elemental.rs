//! `cards/creatures/mv_5/silverglade_elemental.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "a6d5d164-a06a-4383-b9d2-a7a33400631d"
fn silverglade_elemental() -> CardIndex {
    card_index("a6d5d164-a06a-4383-b9d2-a7a33400631d")
}

/// `Silverglade Elemental` (`Coverage::Generated`):
/// "When this creature enters, you may search your library for a Forest card, put that card onto the battlefield, then shuffle."
///
/// Verifies that casting `Silverglade Elemental` triggers its ETB ability and
/// puts a Forest card from the library onto the battlefield when accepted.
#[test]
fn silverglade_elemental_etb_searches_library_for_forest_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9611, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[silverglade_elemental()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let forests_before = engine
        .state()
        .arena
        .iter()
        .filter(|(_, o)| {
            o.zone == Zone::Battlefield
                && o.controller == p0
                && o.card.is_some_and(|c| c.index == forest())
        })
        .count();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, silverglade_elemental());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let chosen_forest = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen_forest],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let forests_after = engine
        .state()
        .arena
        .iter()
        .filter(|(_, o)| {
            o.zone == Zone::Battlefield
                && o.controller == p0
                && o.card.is_some_and(|c| c.index == forest())
        })
        .count();
    assert_eq!(
        forests_after,
        forests_before + 1,
        "Silverglade Elemental puts a Forest card from library onto the battlefield"
    );
}
