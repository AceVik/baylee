#[allow(clippy::wildcard_imports)]
use super::*;

fn vampiric_embrace() -> CardIndex {
    card_index("c7a50ece-4fe4-4e54-9319-b92a4144445d")
}

#[test]
fn vampiric_embrace_pumps_and_grants_flying_to_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[vampiric_embrace()])
        .battlefield(0, &[gray_ogre(), swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    cast_from_hand(&mut engine, p0, vampiric_embrace());
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ogre), (4, 4));
    assert!(keywords(&engine, ogre).contains(KeywordSet::FLYING));
}

#[test]
fn vampiric_embrace_partial_pump_and_flying_apply_only_to_its_host() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[gray_ogre()],
        &[gray_ogre()],
        &[vampiric_embrace()],
        &[],
    );
    let mine = lk::obj(&e, p0, gray_ogre());
    let theirs = lk::obj(&e, p1, gray_ogre());
    lk::float(&mut e, p0, &[(ManaColor::Black, 4)]);
    lk::cast_at(&mut e, p0, vampiric_embrace(), &[mine], &[]);
    assert_eq!(pt(&e, theirs), (2, 2));
    let blocks = lk::attack(&mut e, &[mine]);
    assert!(!lk::may_block(&blocks, theirs, mine));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 16);
}
