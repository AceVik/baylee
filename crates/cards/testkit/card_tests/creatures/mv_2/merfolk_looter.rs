//! Oracle checklist: tap; draw first, then discard exactly one card; assert hand size unchanged and the discarded card reaches its owner's graveyard.
#[allow(clippy::wildcard_imports)]
use super::*;
fn merfolk_looter() -> CardIndex {
    card_index("67362406-b1ca-49e2-800d-9050bfe8742a")
}
#[test]
fn looter_draws_then_discards_one() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[merfolk_looter()])
        .hand(0, &[island()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, merfolk_looter()).unwrap();
    let hand = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let drawn = *e
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .unwrap();
    activate(&mut e, p0, merfolk_looter(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (1, 1));
    assert!(
        options.contains(&drawn),
        "the drawn card is available to discard after the draw"
    );
    let discard = options
        .iter()
        .copied()
        .find(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == island()))
        })
        .expect("known card is discardable");
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discard],
        },
    )
    .expect("discard one");
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), hand);
    assert!(in_graveyard(&e, p0, island()).is_some());
    assert!(is_tapped(&e, source));
}
