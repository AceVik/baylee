//! `cards/creatures/artifacts/mv_1/hullcarver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hullcarver() -> CardIndex {
    card_index("931c2153-e5e5-4642-bff0-f9dadcc24c9a")
}

/// "Deathtouch": a 1/1 blocked by a far larger creature kills it with its
/// single point of damage, and dies to the damage dealt back.
#[test]
fn hullcarver_kills_the_big_creature_that_blocks_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = main_with(&[hullcarver()], &[earth_elemental()]);
    let me = on_battlefield(&engine, p0, hullcarver()).expect("seated");
    let big = on_battlefield(&engine, p1, earth_elemental()).expect("elemental");
    assert!(keywords(&engine, me).contains(KeywordSet::DEATHTOUCH));
    assert!(pt(&engine, big).1 > 1, "toughness far above the 1 damage");
    assert!(pt(&engine, big).0 >= 1, "and able to kill the 1/1");

    attack_with(&mut engine, me, Some(big));
    assert!(
        on_battlefield(&engine, p1, earth_elemental()).is_none(),
        "deathtouch killed the blocker"
    );
    assert!(in_graveyard(&engine, p1, earth_elemental()).is_some());
    assert!(
        on_battlefield(&engine, p0, hullcarver()).is_none(),
        "and the 1/1 took lethal damage in return"
    );
}
