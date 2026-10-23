#[allow(clippy::wildcard_imports)]
use super::*;

fn vile_requiem() -> CardIndex {
    card_index("a72213e7-16f0-4fbe-91d3-750ca51a4336")
}

#[test]
fn vile_requiem_adds_verse_counter_and_destroys_target_nonblack_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[vile_requiem(), swamp(), swamp()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);

    let req_id = on_battlefield(&engine, p0, vile_requiem()).expect("requiem on bf");
    assert_eq!(
        engine
            .state()
            .object(req_id)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::VERSE),
        1
    );

    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre on bf");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, vile_requiem(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
}
