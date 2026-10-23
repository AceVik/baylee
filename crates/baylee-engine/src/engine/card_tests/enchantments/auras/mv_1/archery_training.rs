#[allow(clippy::wildcard_imports)]
use super::*;

fn archery_training() -> CardIndex {
    card_index("acab4fa9-d9ab-4a89-a50f-0cee9d7ca0c3")
}

#[test]
fn archery_training_grants_tap_to_damage_attacking_or_blocking_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[archery_training()])
        .battlefield(0, &[gray_ogre(), plains()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    cast_from_hand(&mut engine, p0, archery_training());
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    let _ = attack_and_collect_blocks(&mut engine, elf, p0);
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    activate(&mut engine, p0, gray_ogre(), crate::choice::GRANTED_ABILITY);
    aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
}
