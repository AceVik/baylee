#[allow(clippy::wildcard_imports)]
use super::*;

fn treefolk_mystic() -> CardIndex {
    card_index("68a6fd00-9cb0-4518-97aa-b7f475da58c1")
}

#[test]
fn treefolk_mystic_destroys_all_auras_attached_to_blocked_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let aura = baylee_core::generated::index::GRANITE_GRIP;
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[aura])
        .battlefield(0, &[gray_ogre(), mountain(), mountain(), mountain()])
        .battlefield(1, &[treefolk_mystic()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, aura);
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
    assert!(on_battlefield(&engine, p0, aura).is_some());

    let mystic = on_battlefield(&engine, p1, treefolk_mystic()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, ogre, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(mystic, ogre)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, aura).is_none());
    assert!(in_graveyard(&engine, p0, aura).is_some());
}
