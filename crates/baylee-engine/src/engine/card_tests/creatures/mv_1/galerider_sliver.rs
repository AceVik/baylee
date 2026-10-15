//! Oracle checklist: all Slivers you control including source have flying; own non-Sliver and an opponent's Sliver do not gain it.
#[allow(clippy::wildcard_imports)]
use super::*;
fn galerider_sliver() -> CardIndex {
    card_index("1ad78038-44f9-4599-84db-fc1a87d9ed45")
}
fn muscle_sliver() -> CardIndex {
    card_index("06e0459f-d691-4470-884a-27c09b6612d2")
}
#[test]
fn galerider_gives_flying_to_its_controllers_slivers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[galerider_sliver(), muscle_sliver(), gray_ogre()])
        .battlefield(1, &[muscle_sliver()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, galerider_sliver()).unwrap();
    let mine = on_battlefield(&e, p0, muscle_sliver()).unwrap();
    let non = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let theirs = on_battlefield(&e, p1, muscle_sliver()).unwrap();
    assert!(keywords(&e, source).contains(KeywordSet::FLYING));
    assert!(keywords(&e, mine).contains(KeywordSet::FLYING));
    assert!(!keywords(&e, non).contains(KeywordSet::FLYING));
    assert!(!keywords(&e, theirs).contains(KeywordSet::FLYING));
}
