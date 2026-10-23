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
