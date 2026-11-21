//! `cards/instants/mv_1/gather_courage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "ae033eb6-47ec-4b9e-9cdb-6d023aca07d1"
fn gather_courage() -> CardIndex {
    card_index("ae033eb6-47ec-4b9e-9cdb-6d023aca07d1")
}

/// Gather Courage ({G}, Convoke): cannot be convoked solely by a non-green
/// creature for {G}, CAN be cast for 0 mana by tapping a green creature via
/// convoke, and gives target creature +2/+2 until end of turn.
#[test]
fn gather_courage_convokes_with_green_creature_and_pumps_until_eot() {
    // Only a red creature and 0 mana: cannot pay {G} via convoke.
    let no_green = lk::game(SEED, &[index::HILL_GIANT], &[], &[gather_courage()], &[]);
    assert!(
        !lk::castable(&no_green, P0, gather_courage()),
        "a red creature alone cannot convoke {{G}}"
    );

    // Untapped green Grizzly Bears pays {G} via convoke with 0 mana in pool.
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[],
        &[gather_courage()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    assert!(lk::castable(&engine, P0, gather_courage()));

    cast_with_floating(&mut engine, P0, gather_courage());
    // Target choice first, then convoke tap choice.
    lk::aim(&mut engine, &[giant], &[]);
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);

    assert!(is_tapped(&engine, bears), "Bears tapped for convoke");
    assert!(!is_tapped(&engine, giant), "Giant was target, not tapped");
    assert_eq!(pt(&engine, giant), (5, 5), "3/3 Hill Giant got +2/+2");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, giant), (3, 3), "+2/+2 wore off at end of turn");
}
