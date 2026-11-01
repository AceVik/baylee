#[allow(clippy::wildcard_imports)]
use super::*;

fn gaea_s_bounty() -> CardIndex {
    card_index("c5a61e61-d5fe-41ef-afd6-ba5e0399b986")
}

#[test]
fn gaea_s_bounty_searches_library_for_up_to_two_forests_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                gaea_s_bounty(),
                island(),
                baylee_core::generated::index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_library_top(&mut engine, p0, island());
    hand_to_library_top(
        &mut engine,
        p0,
        baylee_core::generated::index::GRIZZLY_BEARS,
    );
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    cast_from_hand(&mut engine, p0, gaea_s_bounty());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    while let Pending::ChooseCards { options, max, .. } = engine.pending().clone() {
        assert!(options.iter().all(|id| {
            engine
                .state()
                .object(*id)
                .unwrap()
                .characteristics()
                .subtypes
                .contains(baylee_core::generated::subtypes::land::FOREST)
        }));
        let picks = options.into_iter().take(usize::from(max)).collect();
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: picks })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len()
            > hand_before
    );
}
