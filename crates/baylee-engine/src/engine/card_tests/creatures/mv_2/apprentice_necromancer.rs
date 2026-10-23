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
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    bury(&mut engine, &[ogre]);
    engine.refresh_offer();
    let ogre_gy = in_graveyard(&engine, p0, gray_ogre()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, apprentice_necromancer(), 0);
    aim_at(&mut engine, p0, ogre_gy);
    pass_until(&mut engine, stack_is_empty);

    let reanimated = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert!(keywords(&engine, reanimated).contains(KeywordSet::HASTE));

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, p0, gray_ogre()).is_some());
}
