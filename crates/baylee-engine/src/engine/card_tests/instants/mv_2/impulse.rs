//! `cards/instants/mv_2/impulse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn impulse() -> CardIndex {
    card_index("f6bd2902-7f8b-419e-bbc7-bcab0c1b7e01")
}

/// `Impulse` ({1}{U}, Instant, `Coverage::Implemented`):
/// `Look at the top four cards of your library. Put one of them into your hand
/// and the rest on the bottom of your library in any order.`
/// Four cards are offered, not five; exactly one is taken; the other three go to the bottom, and
/// the card that was fifth from the top is the new top card.
#[test]
fn impulse_looks_at_top_four_puts_one_in_hand_and_rest_on_bottom() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[impulse(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bear_id = in_hand(&engine, p0, grizzly_bears()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            bear_id,
            ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let (fifth, looked) = (
        library[library.len() - 5],
        library[library.len() - 4..].to_vec(),
    );
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, impulse());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1), "one of them");
    assert_eq!(options.len(), 4, "the top four cards");
    assert!(options.iter().all(|card| looked.contains(card)));
    assert!(options.contains(&bear_id));
    assert!(!options.contains(&fifth), "the fifth card is not looked at");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear_id],
            },
        )
        .unwrap();
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        panic!("the rest is arranged, got {:?}", engine.pending())
    };
    assert_eq!(cards.len(), 3, "the three cards that were not taken");
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![cards.clone()],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
    let after = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(after.len(), library.len() - 1);
    assert!(
        after[..3].iter().all(|card| cards.contains(card)),
        "the other three are the bottom three cards of the library"
    );
    assert_eq!(
        *after.last().unwrap(),
        fifth,
        "the fifth card is now on top"
    );
}
