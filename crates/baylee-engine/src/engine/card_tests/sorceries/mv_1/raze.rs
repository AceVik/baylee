#[allow(clippy::wildcard_imports)]
use super::*;

fn raze() -> CardIndex {
    card_index("02fde433-955d-49df-bd72-270b6feac9e7")
}

#[test]
fn raze_sacrifices_your_land_and_destroys_target_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[raze()])
        .battlefield(0, &[mountain()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_mountain = on_battlefield(&engine, p0, mountain()).expect("my mountain");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their forest");
    cast_from_hand(&mut engine, p0, raze());
    aim_at(&mut engine, p0, their_forest);
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![my_mountain],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, mountain()).is_none());
    assert!(on_battlefield(&engine, p1, forest()).is_none());
    assert!(in_graveyard(&engine, p1, forest()).is_some());
}
