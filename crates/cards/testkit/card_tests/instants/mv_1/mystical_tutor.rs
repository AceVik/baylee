//! `cards/instants/mv_1/mystical_tutor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A tutor to the top of the library leaves the card it found on top.
///
/// Mystical Tutor prints "search your library for an instant or sorcery
/// card, reveal it, then shuffle **and put that card on top**", and the
/// search resolution shuffled *after* placing — so the card went on top and
/// was immediately shuffled back into sixty others. Every tutor-to-top in
/// the pool returned a random card.
#[test]
fn a_tutor_to_the_top_leaves_its_card_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(47, counterspell())
        .battlefield(0, &[island()])
        .hand(0, &[mystical_tutor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, mystical_tutor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
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

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(found),
        "the card the tutor found is the one on top",
    );
}
