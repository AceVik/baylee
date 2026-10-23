#[allow(clippy::wildcard_imports)]
use super::*;

fn venomous_fangs() -> CardIndex {
    card_index("a1ee715e-bab5-4edc-89ba-610cf71bb048")
}

#[test]
fn venomous_fangs_destroys_creature_dealt_combat_damage_by_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[venomous_fangs()])
        .battlefield(0, &[gray_ogre(), forest(), forest(), forest()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("elephant on bf");
    cast_from_hand(&mut engine, p0, venomous_fangs());
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    let _ = attack_and_collect_blocks(&mut engine, ogre, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elephant, ogre)],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert!(on_battlefield(&engine, p1, wild_elephant()).is_none());
    assert!(in_graveyard(&engine, p1, wild_elephant()).is_some());
}
