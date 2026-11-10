use super::*;

fn skyshaper() -> CardIndex {
    card_index("b5642460-883c-4e38-8552-9bea4e50d8e8")
}

#[test]
fn skyshaper_sacrifices_to_grant_creatures_you_control_flying_until_eot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skyshaper(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert!(!keywords(&engine, ogre).contains(KeywordSet::FLYING));

    activate(&mut engine, p0, skyshaper(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, skyshaper()).is_none());
    assert!(in_graveyard(&engine, p0, skyshaper()).is_some());
    assert!(keywords(&engine, ogre).contains(KeywordSet::FLYING));
    let enemy = on_battlefield(&engine, PlayerId::new(1), gray_ogre()).unwrap();
    assert!(!keywords(&engine, enemy).contains(KeywordSet::FLYING));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(!keywords(&engine, ogre).contains(KeywordSet::FLYING));
}
