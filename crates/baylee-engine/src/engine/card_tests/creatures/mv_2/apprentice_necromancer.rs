//! `cards/creatures/mv_2/apprentice_necromancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn apprentice_necromancer() -> CardIndex {
    card_index("d1a6aa5f-2ad6-4bc7-82f0-4262f66af730")
}

#[test]
fn apprentice_necromancer_reanimates_creature_with_haste_and_sacrifices_at_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[apprentice_necromancer(), gray_ogre(), swamp()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let opponent = on_battlefield(&engine, PlayerId::new(1), llanowar_elves()).unwrap();
    bury(&mut engine, &[ogre, opponent]);
    hand_to_graveyard(&mut engine, p0, forest());
    engine.refresh_offer();
    let ogre_gy = in_graveyard(&engine, p0, gray_ogre()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, apprentice_necromancer(), 0);
    let opposing = in_graveyard(&engine, PlayerId::new(1), llanowar_elves()).unwrap();
    let land = in_graveyard(&engine, p0, forest()).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("reanimation target")
    };
    assert!(options.contains(&ogre_gy));
    assert!(!options.contains(&opposing));
    assert!(!options.contains(&land));
    aim_at(&mut engine, p0, ogre_gy);
    pass_until(&mut engine, stack_is_empty);

    let reanimated = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert!(keywords(&engine, reanimated).contains(KeywordSet::HASTE));
    assert!(
        in_graveyard(&engine, p0, apprentice_necromancer()).is_some(),
        "the source was paid as a cost"
    );
    crate::engine::card_tests::legends_kit::attack(&mut engine, &[reanimated]);

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, p0, gray_ogre()).is_some());
}
