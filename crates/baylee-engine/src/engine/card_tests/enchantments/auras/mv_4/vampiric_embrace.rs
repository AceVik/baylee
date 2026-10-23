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
