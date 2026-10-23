//! `cards/creatures/mv_1/shadow_guildmage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "cdda63d7-e3ea-455c-b7d4-c6cfdda2703c"
fn shadow_guildmage() -> CardIndex {
    card_index("cdda63d7-e3ea-455c-b7d4-c6cfdda2703c")
}

/// "{R}, {T}: 1 damage to any target and 1 damage to you."
#[test]
fn shadow_guildmage_pings_a_player_and_hurts_its_controller() {
    let mut engine = lk::game(SEED, &[shadow_guildmage()], &[], &[], &[]);
    let mage = lk::obj(&engine, P0, shadow_guildmage());

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::use_ability(&mut engine, P0, shadow_guildmage(), 0, &[], &[P1]);

    assert_eq!(lk::life(&engine, P1), 19);
    assert_eq!(lk::life(&engine, P0), 19);
    assert!(lk::tapped(&engine, mage));
}

/// A creature is a target too, and the second point of damage still goes to
/// the controller; the {T} cost allows one ping a turn.
#[test]
fn shadow_guildmage_pings_a_creature_and_only_once_a_turn() {
    let mut engine = lk::game(
        SEED,
        &[shadow_guildmage()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 2)]);
    lk::use_ability(&mut engine, P0, shadow_guildmage(), 0, &[bears], &[]);

    assert_eq!(lk::damage_on(&engine, bears), 1);
    assert_eq!(lk::life(&engine, P1), 20);
    assert_eq!(lk::life(&engine, P0), 19);
    assert!(
        !lk::offered(&engine, shadow_guildmage(), P0, 0),
        "tapped, it cannot ping again"
    );
}

/// "{U}, {T}: put target creature you control on top of its owner's library"
/// is not on the card (no top-only tuck exists): with only {U} to spend,
/// nothing is offered.
#[test]
fn shadow_guildmage_has_no_blue_ability() {
    let mut engine = lk::game(SEED, &[shadow_guildmage()], &[], &[], &[]);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    assert!(!lk::offered(&engine, shadow_guildmage(), P0, 0));
}
