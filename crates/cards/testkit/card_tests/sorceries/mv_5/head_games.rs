//! `cards/sorceries/mv_5/head_games.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn head_games() -> CardIndex {
    card_index("671a8509-3b6e-499c-a744-ff8b3c2b6e6a")
}

/// Head Games puts target opponent's hand onto their library, lets you search their
/// library for that many cards to put into their hand, then shuffles their library.
#[test]
fn head_games_replaces_opponent_hand_with_chosen_library_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, index::GRIZZLY_BEARS)
        .battlefield(0, &[swamp(); 5])
        .hand(0, &[head_games()])
        .hand(1, &[forest(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib_before = library_size(&engine, p1);
    cast_from_hand(&mut engine, p0, head_games());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange { .. } | Pending::ChooseCards { .. }
        )
    });
    if let Pending::Arrange { player, cards, .. } = engine.pending().clone() {
        engine
            .apply(player, PlayerAction::Arrange { piles: vec![cards] })
            .unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
    }
    let Pending::ChooseCards {
        player,
        min,
        max,
        options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (2, 2), "opponent put 2 cards from hand on top");
    let chosen_bears: Vec<_> = options
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .unwrap()
                .card
                .is_some_and(|c| c.index == index::GRIZZLY_BEARS)
        })
        .take(2)
        .collect();
    assert_eq!(chosen_bears.len(), 2);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: chosen_bears.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let p1_hand = engine.state().zones.list(ZoneLocation::Hand(p1));
    assert_eq!(p1_hand.len(), 2);
    assert!(p1_hand.contains(&chosen_bears[0]));
    assert!(p1_hand.contains(&chosen_bears[1]));
    assert!(in_hand(&engine, p1, forest()).is_none());
    assert!(in_hand(&engine, p1, mountain()).is_none());
    assert_eq!(library_size(&engine, p1), lib_before);
}
