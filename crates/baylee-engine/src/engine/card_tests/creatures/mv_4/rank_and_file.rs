#[allow(clippy::wildcard_imports)]
use super::*;

fn rank_and_file() -> CardIndex {
    card_index("c0b8ab40-3c27-48bd-ae19-fb1a155a0522")
}

#[test]
fn rank_and_file_debuffs_green_creatures_on_etb() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[llanowar_elves(), gray_ogre()])
        .hand(0, &[rank_and_file()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, rank_and_file());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert_eq!(pt(&engine, ogre), (2, 2));
}
