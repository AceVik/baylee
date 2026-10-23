#[allow(clippy::wildcard_imports)]
use super::*;

fn remembrance() -> CardIndex {
    card_index("02d0b0fd-ff73-4e26-be44-d85c743d51e4")
}

#[test]
fn remembrance_searches_library_for_creature_when_nontoken_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, gray_ogre())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[remembrance(), gray_ogre(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear_id = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, bear_id);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, gray_ogre()).is_some());
}
