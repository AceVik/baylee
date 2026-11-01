//! `cards/creatures/mv_4/reckless_embermage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5771296a-16f8-423e-9b83-8e6f687cbb34"
fn reckless_embermage() -> CardIndex {
    card_index("5771296a-16f8-423e-9b83-8e6f687cbb34")
}

#[test]
fn reckless_embermage_deals_1_damage_to_any_target_and_1_damage_to_itself() {
    let mut engine = lk::game(SEED, &[reckless_embermage()], &[], &[], &[]);
    let mage = lk::obj(&engine, P0, reckless_embermage());

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
    );
    lk::use_ability(&mut engine, P0, reckless_embermage(), 0, &[], &[P1]);

    assert_eq!(lk::life(&engine, P1), 19);
    assert_eq!(lk::damage_on(&engine, mage), 1);
}

/// "Any target" includes a creature, and the 1 damage to itself is real: the
/// second activation kills a 2/2 Embermage, together with the 2/2 it aimed at.
#[test]
fn reckless_embermage_pings_a_creature_and_hurts_itself_each_time() {
    let mut engine = lk::game(
        SEED,
        &[reckless_embermage()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let mage = lk::obj(&engine, P0, reckless_embermage());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
    );
    lk::use_ability(&mut engine, P0, reckless_embermage(), 0, &[bears], &[]);
    assert_eq!(lk::damage_on(&engine, bears), 1);
    assert_eq!(lk::damage_on(&engine, mage), 1);
    assert_eq!(lk::life(&engine, P1), 20, "the opponent was not the target");
    assert_eq!(
        lk::life(&engine, P0),
        20,
        "the damage to itself is not to its controller"
    );

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
    );
    lk::use_ability(&mut engine, P0, reckless_embermage(), 0, &[bears], &[]);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(
        in_graveyard(&engine, P0, reckless_embermage()).is_some(),
        "two points of damage to itself kill a 2/2"
    );
}
