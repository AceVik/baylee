use super::*;
use baylee_core::generated::index;

fn plaguebearer() -> CardIndex {
    card_index("f8cc1dc4-740d-4c5b-ac51-7355730a6595")
}

#[test]
fn plaguebearer_destroys_target_nonblack_creature_with_cmc_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                plaguebearer(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[gray_ogre(), grizzly_bears(), index::GROLLUB])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, plaguebearer(), 0);
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    let wrong_value = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let black = on_battlefield(&engine, p1, index::GROLLUB).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("targets")
    };
    assert!(!options.contains(&wrong_value));
    assert!(!options.contains(&black));
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
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "X=3 costs three twice plus one black"
    );
}
