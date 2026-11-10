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

/// The branch the first test declines: "You may have that player shuffle."
/// Aimed at the opponent and answered yes, the opponent's library is shuffled
/// after the caster arranged its top three (the chosen order does not stand),
/// no card is lost, and the caster's own library is not touched.
#[test]
fn natural_selection_shuffles_the_targeted_players_library_when_asked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[natural_selection()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ours_before: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let theirs_before: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p1)).clone();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, natural_selection());
    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the opponent is a legal target player");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { player, cards, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the arrangement")
    };
    assert_eq!(player, p0, "the caster arranges");
    let top_three: Vec<ObjectId> = theirs_before.iter().rev().take(3).copied().collect();
    assert_eq!(cards, top_three, "the opponent's top three, not ours");
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
        panic!("expected the shuffle question, got {:?}", engine.pending())
    };
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let after: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p1)).clone();
    assert_eq!(after.len(), theirs_before.len(), "no card is lost");
    assert!(after.iter().all(|id| theirs_before.contains(id)));
    let new_top: Vec<ObjectId> = after.iter().rev().take(3).copied().collect();
    assert_ne!(
        new_top, reversed,
        "answered yes: the chosen order was shuffled away"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).clone(),
        ours_before,
        "and the shuffle was theirs, not ours"
    );
}
