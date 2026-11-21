//! `cards/instants/mv_3/strength_of_night.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "0045cf16-86dd-4417-ae36-88ca63b30c26"
fn strength_of_night() -> CardIndex {
    card_index("0045cf16-86dd-4417-ae36-88ca63b30c26")
}

/// Strength of Night ({2}{G}, Kicker {B}): unkicked gives creatures you control
/// +1/+1 until end of turn (leaving opponent's creatures untouched), then wears
/// off next turn.
#[test]
fn strength_of_night_unkicked_gives_your_creatures_plus_one_plus_one() {
    let mut engine = lk::game(
        SEED,
        &[index::SCATHE_ZOMBIES, index::GRIZZLY_BEARS],
        &[index::SCATHE_ZOMBIES],
        &[strength_of_night()],
        &[],
    );
    let my_zombie = lk::obj(&engine, P0, index::SCATHE_ZOMBIES);
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_zombie = lk::obj(&engine, P1, index::SCATHE_ZOMBIES);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Green, 3), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, strength_of_night());
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, my_zombie), (3, 3));
    assert_eq!(pt(&engine, my_bears), (3, 3));
    assert_eq!(pt(&engine, their_zombie), (2, 2));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_zombie), (2, 2));
    assert_eq!(pt(&engine, my_bears), (2, 2));
}

/// Kicked for {B}: your Zombies get +3/+3 total (+1/+1 plus +2/+2), your
/// non-Zombies get +1/+1, and opponent's Zombies stay untouched.
#[test]
fn strength_of_night_kicked_gives_your_zombies_extra_plus_two_plus_two() {
    let mut engine = lk::game(
        SEED,
        &[index::SCATHE_ZOMBIES, index::GRIZZLY_BEARS],
        &[index::SCATHE_ZOMBIES],
        &[strength_of_night()],
        &[],
    );
    let my_zombie = lk::obj(&engine, P0, index::SCATHE_ZOMBIES);
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_zombie = lk::obj(&engine, P1, index::SCATHE_ZOMBIES);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Green, 3), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, strength_of_night());
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P0), 0, "all {{2}}{{G}}{{B}} spent");
    assert_eq!(pt(&engine, my_zombie), (5, 5), "own Zombie gets +3/+3");
    assert_eq!(pt(&engine, my_bears), (3, 3), "own non-Zombie gets +1/+1");
    assert_eq!(
        pt(&engine, their_zombie),
        (2, 2),
        "opponent Zombie untouched"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_zombie), (2, 2));
}
