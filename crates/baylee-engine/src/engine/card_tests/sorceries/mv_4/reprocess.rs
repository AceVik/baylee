#[allow(clippy::wildcard_imports)]
use super::*;

fn reprocess() -> CardIndex {
    card_index("27177cb0-1670-488e-82be-4610da5a7de3")
}

#[test]
fn reprocess_sacrifices_permanent_and_draws_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[reprocess()])
        .battlefield(0, &[gray_ogre(), swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    let bear_id = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    cast_from_hand(&mut engine, p0, reprocess());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bear_id],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before
    );
}

#[test]
fn reprocess_partial_may_decline_and_cannot_sacrifice_opponents_or_enchantments() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[gray_ogre(), index::CRUSADE],
        &[wild_elephant()],
        &[reprocess()],
        &[],
    );
    let mine = lk::obj(&e, p0, gray_ogre());
    let theirs = lk::obj(&e, p1, wild_elephant());
    let enchantment = lk::obj(&e, p0, index::CRUSADE);
    lk::float(&mut e, p0, &[(ManaColor::Black, 4)]);
    cast_with_floating(&mut e, p0, reprocess());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending() else {
        unreachable!()
    };
    assert!(
        options.contains(&mine) && !options.contains(&theirs) && !options.contains(&enchantment)
    );
    e.apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    lk::settle(&mut e);
    assert!(
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .is_empty()
    );
    assert!(on_battlefield(&e, p0, gray_ogre()).is_some());
}
