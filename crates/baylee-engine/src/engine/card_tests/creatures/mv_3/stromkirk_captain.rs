//! Oracle checklist: Captain itself has first strike; other Vampires you control get +1/+1 and first strike; non-Vampire and opposing Vampire are excluded; “other” excludes Captain itself from the bonus.
#[allow(clippy::wildcard_imports)]
use super::*;
fn stromkirk_captain() -> CardIndex {
    card_index("599c3fd6-3309-4b5d-adec-9c4062848ad5")
}
fn arrogant_vampire() -> CardIndex {
    card_index("5edfe083-391b-41ae-ac4d-d648042cfa5e")
}
#[test]
fn captain_grants_stats_and_first_strike_to_other_controlled_vampires() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[stromkirk_captain(), arrogant_vampire(), gray_ogre()])
        .battlefield(1, &[arrogant_vampire()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let captain = on_battlefield(&e, p0, stromkirk_captain()).unwrap();
    let mine = on_battlefield(&e, p0, arrogant_vampire()).unwrap();
    let non = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let theirs = on_battlefield(&e, p1, arrogant_vampire()).unwrap();
    assert!(keywords(&e, captain).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(pt(&e, captain), (2, 2));
    assert_eq!(pt(&e, mine), (5, 4));
    assert!(keywords(&e, mine).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(pt(&e, non), (2, 2));
    assert!(!keywords(&e, non).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(pt(&e, theirs), (4, 3));
    assert!(!keywords(&e, theirs).contains(KeywordSet::FIRST_STRIKE));
}
