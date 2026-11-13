//! `cards/enchantments/mv_1/mirri_s_guile.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The optional upkeep trigger looks at exactly three cards and puts those
/// same cards back in the order its controller chooses, without drawing one.
#[test]
fn mirri_s_guile_asks_at_upkeep_and_reorders_exactly_the_top_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(399, forest())
        .battlefield(0, &[mirri_s_guile()])
        .hand(0, &[forest(), island(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    let stacked = baylee_card_testkit::card_tests::partials_w1b_kit::stack_library(
        &mut engine,
        p0,
        &[forest(), island(), mountain()],
    );

    let before = library_size(&engine, p0);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!(cards, stacked, "the top three, top card first");
    assert_eq!(prompt, ArrangePrompt::Order);
    assert_eq!(
        piles,
        vec![ArrangePile::all_of(ArrangePlace::LibraryTop, 3)]
    );

    let mut reversed = cards;
    reversed.reverse();
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![reversed.clone()],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let library = engine.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(library.len(), before, "nothing left the library");
    assert_eq!(
        library.iter().rev().take(3).copied().collect::<Vec<_>>(),
        reversed,
        "the chosen top-to-bottom order is preserved"
    );
}
