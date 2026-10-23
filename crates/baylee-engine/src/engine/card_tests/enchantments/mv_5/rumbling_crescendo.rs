#[allow(clippy::wildcard_imports)]
use super::*;

fn rumbling_crescendo() -> CardIndex {
    card_index("2e7e19c2-5399-433d-9653-d956f11838f5")
}

#[test]
fn rumbling_crescendo_adds_verse_counter_and_destroys_target_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[rumbling_crescendo(), mountain()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let crescendo_id = on_battlefield(&engine, p0, rumbling_crescendo()).expect("crescendo on bf");
    assert_eq!(
        engine
            .state()
            .object(crescendo_id)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::VERSE),
        1
    );

    let their_forest = on_battlefield(&engine, p1, forest()).expect("forest on bf");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rumbling_crescendo(), 1);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![their_forest],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, forest()).is_none());
    assert!(in_graveyard(&engine, p1, forest()).is_some());
}
