//! `cards/creatures/mv_7/rune_scarred_demon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn rune_scarred_demon() -> CardIndex {
    card_index("14aefe99-fb60-4f1c-a71f-7ffbe94c8b13")
}

/// Rune-Scarred Demon — a 6/6 flier for {5}{B}{B}: "search your library for a
/// card, put it into your hand, then shuffle". The search asks which card,
/// and the one chosen is in hand afterwards, one fewer in the library.
#[test]
fn rune_scarred_demon_tutors_a_card_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(swamp(), 7), &[rune_scarred_demon()], &[]);
    pay_exactly_and_cast(&mut engine, rune_scarred_demon());
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library = library_size(&engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass stopped here")
    };
    assert_eq!(
        library,
        options.len(),
        "any card of the library is on offer"
    );
    assert_eq!((min, max), (1, 1), "a mandatory one card");
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card of the library");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1
    );
    assert_eq!(library_size(&engine, p0), library - 1);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen)
    );
    let demon = on_battlefield(&engine, p0, rune_scarred_demon()).expect("out");
    assert_body(&engine, demon, (6, 6), KeywordSet::FLYING);
}
