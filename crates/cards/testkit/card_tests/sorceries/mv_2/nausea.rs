//! `cards/sorceries/mv_2/nausea.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn nausea() -> CardIndex {
    card_index("2b8abae2-dd15-41ee-81ae-6a463723b43e")
}

#[test]
fn nausea_gives_all_creatures_minus_one_minus_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[swamp(), swamp(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[nausea()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, nausea());
    pass_until(&mut engine, stack_is_empty);

    let mine = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert_eq!(pt(&engine, bears), (1, 1));
    assert_eq!(pt(&engine, mine), (1, 1));
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(pt(&engine, mine), (2, 2));
}
