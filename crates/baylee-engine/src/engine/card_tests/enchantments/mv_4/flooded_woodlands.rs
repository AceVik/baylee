//! `cards/enchantments/mv_4/flooded_woodlands.rs`, played.
//!
//! Oracle:
//! - "Green creatures can't attack unless their controller sacrifices a land of their choice for each green creature they control that's attacking."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flooded_woodlands() -> CardIndex {
    card_index("413172aa-b508-4e67-b68e-6a749d2e8130")
}

/// The opponent's Woodlands tax P0's green attacker one of P0's own lands,
/// as attackers are declared; the white attacker beside it costs nothing,
/// and the Woodlands' controller keeps its lands.
#[test]
fn flooded_woodlands_charges_a_land_for_a_green_attacker_and_nothing_for_a_white_one() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS, index::FOREST],
        &[flooded_woodlands(), index::FOREST],
        &[],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let forest = lk::obj(&engine, P0, index::FOREST);

    let _ = lk::attack(&mut engine, &[bears, lions]);

    assert_eq!(
        lk::zone_of(&engine, forest),
        Zone::Graveyard,
        "the green Bears' attack cost P0 its Forest"
    );
    assert!(
        all_on_battlefield(&engine, P0, index::FOREST).is_empty(),
        "one land for one green attacker, and the white Lions cost none"
    );
    assert_eq!(
        all_on_battlefield(&engine, P1, index::FOREST).len(),
        1,
        "\"their controller\" sacrifices, not the Woodlands' controller"
    );
    assert!(lk::tapped(&engine, bears), "the Bears attack");
    assert!(lk::tapped(&engine, lions), "and so do the Lions");
}

/// With no land to sacrifice, a green creature can't attack; a white one
/// still may.
#[test]
fn flooded_woodlands_keeps_a_green_creature_home_without_a_land() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
        &[flooded_woodlands()],
        &[],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);

    let offered = lk::attackers_offered(&mut engine);
    assert!(
        !offered.contains(&bears),
        "no land to sacrifice, no green attacker"
    );
    assert!(offered.contains(&lions), "the white Lions are not taxed");
}

/// "For each green creature they control that's attacking", and "green
/// creatures" means its own controller's too: two green attackers under the
/// Woodlands' own controller cost two of three Forests.
#[test]
fn flooded_woodlands_charges_one_land_for_each_green_attacker_its_own_controller_s_included() {
    let mut engine = lk::game(
        SEED,
        &[
            flooded_woodlands(),
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::FOREST,
            index::FOREST,
            index::FOREST,
        ],
        &[],
        &[],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let elves = lk::obj(&engine, P0, index::LLANOWAR_ELVES);

    let _ = lk::attack(&mut engine, &[bears, elves]);

    assert_eq!(
        all_on_battlefield(&engine, P0, index::FOREST).len(),
        1,
        "two green attackers, two Forests sacrificed"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        2
    );
}
