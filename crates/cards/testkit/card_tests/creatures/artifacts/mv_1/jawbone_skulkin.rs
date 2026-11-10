//! `cards/creatures/artifacts/mv_1/jawbone_skulkin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn jawbone_skulkin() -> CardIndex {
    card_index("1cbcc073-1645-4d7f-98f4-7309d7b35692")
}

/// "{2}: Target red creature gains haste until end of turn." The menu holds
/// the red Ogre and neither the green Bears nor the colorless Skulkin, and
/// the haste lapses with the turn.
#[test]
fn jawbone_skulkin_gives_a_red_creature_haste() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[jawbone_skulkin(), gray_ogre(), grizzly_bears()], &[]);
    let me = on_battlefield(&engine, p0, jawbone_skulkin()).expect("seated");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("bears");
    assert!(!keywords(&engine, ogre).contains(KeywordSet::HASTE));

    float_mana(&mut engine, ManaColor::Colorless, 2);
    activate(&mut engine, p0, jawbone_skulkin(), 0);
    let options = aim_at(&mut engine, p0, ogre);
    assert!(options.contains(&ogre));
    assert!(!options.contains(&bears), "green is not red");
    assert!(!options.contains(&me), "colorless is not red");
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, ogre).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, bears).contains(KeywordSet::HASTE));

    pass_until(&mut engine, |e| {
        e.state().turn.active == PlayerId::new(1) && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        !keywords(&engine, ogre).contains(KeywordSet::HASTE),
        "until end of turn"
    );
}
