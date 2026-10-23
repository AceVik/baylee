//! `cards/enchantments/mv_5/precognition.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn precognition() -> CardIndex {
    card_index("6d1a9d06-028f-487a-b490-27100c128ea3")
}

/// `Precognition` ({4}{U}, Enchantment, `Coverage::Implemented`):
/// `At the beginning of your upkeep, you may look at the top card of target
/// opponent's library. If you do, you may put that card on the bottom of that
/// player's library.`
#[test]
fn precognition_looks_at_opponents_top_card_on_upkeep_and_may_bottom_or_keep_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[precognition()])
        .start();
    keep_mulligans(&mut engine);

    let top_of_p1 = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .unwrap();

    // On p0's upkeep, Precognition triggers and asks for target opponent.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { player, cards, .. } = engine.pending().clone() else {
        panic!("expected Arrange for ScryFor");
    };
    assert_eq!(player, p0);
    assert_eq!(cards, vec![top_of_p1]);

    // Put the card on the bottom of p1's library (`took` = bottom pile for scry).
    engine.apply(p0, look_answer(&cards, &[top_of_p1])).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p1))
            .first()
            .copied(),
        Some(top_of_p1)
    );
}

#[test]
fn precognition_may_decline_the_look_or_keep_the_top_card() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for look in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[precognition()])
            .start();
        keep_mulligans(&mut e);
        let top = *e
            .state()
            .zones
            .list(ZoneLocation::Library(p1))
            .last()
            .unwrap();
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        assert!(
            e.apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p0]
                }
            )
            .is_err(),
            "target opponent excludes you"
        );
        lk::aim(&mut e, &[], &[p1]);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        e.apply(p0, PlayerAction::YesNo(look)).unwrap();
        if look {
            pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
            let Pending::Arrange { player, cards, .. } = e.pending().clone() else {
                panic!("looked card");
            };
            assert_eq!(player, p0);
            e.apply(p0, look_answer(&cards, &[])).unwrap();
        }
        lk::settle(&mut e);
        assert_eq!(
            e.state()
                .zones
                .list(ZoneLocation::Library(p1))
                .last()
                .copied(),
            Some(top)
        );
    }
}
