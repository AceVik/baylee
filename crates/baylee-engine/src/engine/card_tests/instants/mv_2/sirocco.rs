//! `cards/instants/mv_2/sirocco.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sirocco() -> CardIndex {
    card_index("a9bdf857-0c6b-4afa-afbb-aff7e4b61b09")
}

/// Sirocco (`{1}{R}`, `Coverage::Partial`): reveals target player's hand and
/// makes them discard a blue instant card.
#[test]
fn sirocco_reveals_hand_and_discards_blue_instant() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[sirocco()])
        .hand(1, &[counterspell(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sirocco());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(options.len(), 1);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, counterspell()).is_some());
    assert!(in_hand(&engine, p1, grizzly_bears()).is_some());
}
