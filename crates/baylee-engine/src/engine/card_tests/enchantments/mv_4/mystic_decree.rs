//! `cards/enchantments/mv_4/mystic_decree.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn mystic_decree() -> CardIndex {
    card_index("06eb8b90-aad8-41c1-bbc5-85ded2fe76bc")
}

fn river_bear() -> CardIndex {
    card_index("568c4d02-501e-44ce-871c-adf9e4b9d8d6")
}

/// "All creatures lose flying and islandwalk": a flyer of either side loses
/// flying, an islandwalker loses islandwalk, and with the Decree gone both
/// are back (the layer system, not a one-time removal).
#[test]
fn mystic_decree_strips_flying_and_islandwalk_from_all_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mystic_decree(), serra_angel(), river_bear()])
        .battlefield(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let angel = on_battlefield(&engine, p0, serra_angel()).expect("seated");
    let foe_angel = on_battlefield(&engine, p1, serra_angel()).expect("seated");
    let bear = on_battlefield(&engine, p0, river_bear()).expect("seated");
    let flying = baylee_cards_dsl::KeywordSet::FLYING;
    let islandwalk = baylee_cards_dsl::KeywordSet::ISLANDWALK;
    assert!(
        !keywords(&engine, angel).contains(flying),
        "mine lost flying"
    );
    assert!(
        !keywords(&engine, foe_angel).contains(flying),
        "an opposing flyer lost it too"
    );
    assert!(
        !keywords(&engine, bear).contains(islandwalk),
        "islandwalk is gone"
    );

    let decree = on_battlefield(&engine, p0, mystic_decree()).expect("seated");
    crate::sba::destroy(engine.dev_state_mut(p0).expect("the harness"), decree);
    resync(&mut engine);
    assert!(
        keywords(&engine, angel).contains(flying),
        "with the Decree gone the Angel flies again"
    );
    assert!(keywords(&engine, bear).contains(islandwalk));
}
