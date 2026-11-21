//! `cards/sorceries/mv_2/lava_blister.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lava_blister() -> CardIndex {
    card_index("3260bede-d78d-41a6-8fe1-95ce14a2ba8c")
}

/// Lava Blister (3260bede-d78d-41a6-8fe1-95ce14a2ba8c) — {1}{R} Sorcery:
/// "Destroy target nonbasic land unless its controller has Lava Blister deal 6
/// damage to them."
#[test]
fn lava_blister_targets_nonbasic_land_and_destroys_unless_controller_takes_six() {
    let mut declined = lk::game(
        SEED,
        &[mountain(), mountain()],
        &[badlands(), forest()],
        &[lava_blister()],
        &[],
    );
    let nonbasic = lk::obj(&declined, P1, badlands());
    let basic = lk::obj(&declined, P1, forest());

    cast_from_hand(&mut declined, P0, lava_blister());
    let menu = aim_at(&mut declined, P0, nonbasic);
    assert_eq!(menu, vec![nonbasic], "basic lands excluded");
    assert!(!menu.contains(&basic));

    pass_until(&mut declined, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = declined.pending().clone() else {
        unreachable!();
    };
    assert_eq!(player, P1);
    lk::yes_no(&mut declined, false);
    lk::settle(&mut declined);
    assert_eq!(lk::zone_of(&declined, nonbasic), Zone::Graveyard);
    assert_eq!(lk::life(&declined, P1), 20);

    let mut accepted = lk::game(
        SEED,
        &[mountain(), mountain()],
        &[badlands()],
        &[lava_blister()],
        &[],
    );
    let saved = lk::obj(&accepted, P1, badlands());
    cast_from_hand(&mut accepted, P0, lava_blister());
    aim_at(&mut accepted, P0, saved);
    pass_until(&mut accepted, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut accepted, true);
    lk::settle(&mut accepted);
    assert_eq!(lk::zone_of(&accepted, saved), Zone::Battlefield);
    assert_eq!(lk::life(&accepted, P1), 14, "took 6 damage to save land");
}
