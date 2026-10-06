//! `cards/instants/mv_1/natural_selection.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Natural Selection: "Look at the top three cards of target player's
/// library, then put them back in any order. You may have that player
/// shuffle their library instead." Declining the shuffle leaves the chosen
/// order standing — the mandatory first sentence, proven on its own.
#[test]
fn natural_selection_reorders_the_top_three_and_may_decline_the_shuffle() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[natural_selection()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, natural_selection());
    engine
        .apply(p0, PlayerAction::ChoosePlayer(p0))
        .expect("p0 targets itself");
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
        panic!("expected an arrangement, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ArrangePrompt::Order, "\"in any order\"");
    assert_eq!(
        piles,
        vec![ArrangePile::all_of(ArrangePlace::LibraryTop, 3)]
    );
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top_three: Vec<ObjectId> = library.iter().rev().take(3).copied().collect();
    assert_eq!(cards, top_three);

    let mut reversed = cards.clone();
    reversed.reverse();
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![reversed.clone()],
            },
        )
        .unwrap();

    let Pending::YesNo {
        prompt: YesNoPrompt::MayDo,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected \"you may have that player shuffle instead\", got {:?}",
            engine.pending()
        )
    };
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let after = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let new_top: Vec<ObjectId> = after.iter().rev().take(3).copied().collect();
    assert_eq!(
        new_top, reversed,
        "declined the shuffle: the chosen order stands"
    );
}
