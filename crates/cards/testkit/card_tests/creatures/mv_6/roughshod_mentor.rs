//! Oracle checklist: green creatures you control have trample; own green creature and green source receive it, own non-green creature and opposing green creature do not.
#[allow(clippy::wildcard_imports)]
use super::*;
fn roughshod_mentor() -> CardIndex {
    card_index("ec43fb64-122d-4fc5-9fe0-b7913373f7a4")
}
#[test]
fn mentor_grants_trample_to_only_its_controllers_green_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[roughshod_mentor(), llanowar_elves(), gray_ogre()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, roughshod_mentor()).unwrap();
    let green = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let other = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let theirs = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    assert!(keywords(&e, source).contains(KeywordSet::TRAMPLE));
    assert!(keywords(&e, green).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&e, other).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&e, theirs).contains(KeywordSet::TRAMPLE));
}
