//! `cards/instants/mv_4/waterlogged_teachings.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Waterlogged Teachings` // `Inundated Archive` (`Coverage::Implemented`): "Search your
/// library for an instant card or a card with flash, reveal it, put it into your hand,
/// then shuffle. // This land enters tapped. {T}: Add {U} or {B}."
///
/// Marked `Coverage::Implemented`, casting the front face for `{3}{U/B}` triggers
/// `Effect::SearchLibrary` for an instant or flash card. With a library filled with
/// `counterspell()`, `Pending::ChooseCards` offers the instant card, which is chosen
/// and added to hand, while `Waterlogged Teachings` moves to the graveyard.
#[test]
fn waterlogged_teachings_searches_library_for_instant_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(471, counterspell())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[waterlogged_teachings()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, waterlogged_teachings());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected search prompt, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (1, 1), "mandatory search for one card");
    let found = *options
        .first()
        .expect("the library is full of Counterspells");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, counterspell()).is_some(),
        "the searched instant was put into hand"
    );
    assert!(
        in_graveyard(&engine, p0, waterlogged_teachings()).is_some(),
        "Waterlogged Teachings resolved and moved to graveyard"
    );
}
