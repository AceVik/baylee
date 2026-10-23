//! `cards/artifacts/mv_3/caltrops.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn caltrops() -> CardIndex {
    card_index("b13f97b3-56ae-4131-a93b-b162c8ff7cca")
}

#[test]
fn caltrops_deals_one_damage_to_each_attacking_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[caltrops(), llanowar_elves(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    let own_ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert_eq!(
        engine.state().object(own_ogre).unwrap().damage,
        0,
        "a nonattacker is untouched"
    );
    reach_their_main_phase(&mut engine, p1);
    let enemy_ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, enemy_ogre, p0);
    assert_eq!(
        engine.state().object(enemy_ogre).unwrap().damage,
        1,
        "opponents' attackers also take damage"
    );
    assert_eq!(engine.state().object(own_ogre).unwrap().damage, 0);
}
