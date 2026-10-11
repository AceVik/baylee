//! `cards/lands/utility/fertile_thicket.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fertile Thicket: "This land enters tapped." / "When this land enters, you may look at the top five cards of your library..." / "{T}: Add {G}."
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for green mana.
#[test]
fn fertile_thicket_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(333, forest())
        .hand(0, &[fertile_thicket()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, fertile_thicket());
    assert!(entered_tapped(&engine, land));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, fertile_thicket(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn thicket_keeps_one_basic_on_top_and_orders_the_other_four_on_bottom() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[fertile_thicket(), index::GRIZZLY_BEARS])
        .commander(0, &[index::MOUNTAIN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let chosen = e.state().commanders[0][0].object;
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            chosen,
            ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    let bear = hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
    let hand_count = e.state().zones.list(ZoneLocation::Hand(p0)).len() - 1;
    play_land(&mut e, p0, fertile_thicket());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert!(options.contains(&chosen));
    assert!(!options.contains(&bear));
    assert_eq!((*min, *max), (0, 1));
    assert_eq!(*prompt, ChoicePrompt::PutBackOnTop);
    assert_eq!(e.inspected_cards().len(), 5);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![chosen],
        },
    )
    .unwrap();
    let Pending::Arrange { cards, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(cards.len(), 4);
    assert!(cards.contains(&bear));
    assert!(!cards.contains(&chosen));
    let ordered = cards.into_iter().rev().collect::<Vec<_>>();
    e.apply(
        p0,
        PlayerAction::Arrange {
            piles: vec![ordered.clone()],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let library = e.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(library.last(), Some(&chosen));
    assert_eq!(
        &library[..4],
        ordered.iter().rev().copied().collect::<Vec<_>>().as_slice()
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_count
    );
}
