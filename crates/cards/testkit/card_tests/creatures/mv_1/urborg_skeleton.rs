//! `cards/creatures/mv_1/urborg_skeleton.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn urborg_skeleton() -> CardIndex {
    card_index("712d7eb4-4044-4852-a5da-57635d3086b0")
}

/// Urborg Skeleton ({B} 0/1 Skeleton):
/// "Kicker {3}; {B}: Regenerate this creature. If this creature was kicked, it
/// enters with a +1/+1 counter on it."
#[test]
fn urborg_skeleton_regenerates_for_black_and_enters_with_counter_when_kicked() {
    // Unkicked: 0/1 with 0 counters; {B} regenerates it through Shock.
    let mut e = lk::game(SEED, &[], &[], &[urborg_skeleton(), index::SHOCK], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 4)]);
    cast_with_floating(&mut e, lk::P0, urborg_skeleton());
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);

    let skel = lk::obj(&e, lk::P0, urborg_skeleton());
    assert_eq!(pt(&e, skel), (0, 1));
    assert_eq!(counters_on(&e, skel, CounterKind::P1P1), 0);

    lk::use_ability(&mut e, lk::P0, urborg_skeleton(), 0, &[], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::SHOCK, &[skel], &[]);
    assert!(on_battlefield(&e, lk::P0, urborg_skeleton()).is_some());
    assert!(lk::tapped(&e, skel));
    assert_eq!(lk::damage_on(&e, skel), 0);

    // Kicked: pays {3}{B}, enters with 1 +1/+1 counter as 1/2.
    let mut e2 = lk::game(SEED, &[], &[], &[urborg_skeleton()], &[]);
    lk::float(&mut e2, lk::P0, &[(ManaColor::Black, 4)]);
    cast_with_floating(&mut e2, lk::P0, urborg_skeleton());
    lk::yes_no(&mut e2, true);
    lk::settle(&mut e2);
    let kicked = lk::obj(&e2, lk::P0, urborg_skeleton());
    assert_eq!(counters_on(&e2, kicked, CounterKind::P1P1), 1);
    assert_eq!(pt(&e2, kicked), (1, 2));
}
