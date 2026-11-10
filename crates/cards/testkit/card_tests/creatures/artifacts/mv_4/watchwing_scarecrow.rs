//! `cards/creatures/artifacts/mv_4/watchwing_scarecrow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn watchwing_scarecrow() -> CardIndex {
    card_index("24bd190c-04bf-44d3-97ba-9dafd9adf7ae")
}

/// A board of the Scarecrow and `mine` for seat 0 and `theirs` for seat 1.
fn board(mine: &[CardIndex], theirs: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut seat0 = vec![watchwing_scarecrow()];
    seat0.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &seat0)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let scarecrow = on_battlefield(&engine, p0, watchwing_scarecrow()).expect("seated");
    (engine, scarecrow)
}

/// Watchwing Scarecrow — {4} 2/4: "This creature has vigilance as long as
/// you control a white creature. This creature has flying as long as you
/// control a blue creature." A Serra Zealot (white) gives vigilance and not
/// flying; and it stops when the Zealot dies.
#[test]
fn watchwing_scarecrow_has_vigilance_beside_a_white_creature() {
    let p0 = PlayerId::new(0);
    let (mut engine, scarecrow) = board(&[serra_zealot()], &[]);
    assert!(keywords(&engine, scarecrow).contains(KeywordSet::VIGILANCE));
    assert!(!keywords(&engine, scarecrow).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, scarecrow), (2, 4));

    let zealot = on_battlefield(&engine, p0, serra_zealot()).expect("seated");
    kill(&mut engine, zealot);
    assert!(
        !keywords(&engine, scarecrow).contains(KeywordSet::VIGILANCE),
        "no white creature any more"
    );
}

/// A Giant Tortoise (blue) gives flying and not vigilance.
#[test]
fn watchwing_scarecrow_flies_beside_a_blue_creature() {
    let (engine, scarecrow) = board(&[giant_tortoise()], &[]);
    assert!(keywords(&engine, scarecrow).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, scarecrow).contains(KeywordSet::VIGILANCE));
}

/// Neither on its own, and an opponent's white or blue creature counts for
/// nothing ("you control").
#[test]
fn watchwing_scarecrow_needs_a_creature_of_its_own_side() {
    let (engine, scarecrow) = board(&[grizzly_bears()], &[serra_zealot(), giant_tortoise()]);
    assert_eq!(keywords(&engine, scarecrow), KeywordSet::EMPTY);
}
