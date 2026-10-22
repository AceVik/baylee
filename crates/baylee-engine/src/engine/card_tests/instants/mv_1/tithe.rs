//! `cards/instants/mv_1/tithe.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tithe() -> CardIndex {
    card_index("318304f5-0929-4cd3-97e6-08ac1ac48aa6")
}

/// `Tithe` ({W}, Instant, `Coverage::Partial`) targets an opponent and searches
/// your library for up to two Plains cards into your hand when an opponent
/// controls more lands than you.
#[test]
fn tithe_searches_for_two_plains_when_opponent_controls_more_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[tithe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, tithe());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0], options[1]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 2);
}
