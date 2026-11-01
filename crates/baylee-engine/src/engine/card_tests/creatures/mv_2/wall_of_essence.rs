//! Defender is played; the unsupported received-combat-damage trigger is omitted.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;
#[test]
fn defender_cannot_attack_and_bolt_has_no_invented_noncombat_benefit() {
    let subject = card_index("1374ac18-01bd-452f-af72-a5f96285ed69");
    let mut e = lk::game(SEED, &[subject], &[], &[lightning_bolt()], &[]);
    let wall = lk::obj(&e, lk::P0, subject);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, lightning_bolt(), &[wall], &[]);
    assert_eq!(lk::life(&e, lk::P0), 20);
    assert_eq!(lk::life(&e, lk::P1), 20);
    assert_eq!(lk::damage_on(&e, wall), 3);
    assert!(!lk::attackers_offered(&mut e).contains(&wall));
}
#[test]
fn supported_wall_blocks_and_survives_a_three_power_attacker() {
    let subject = card_index("1374ac18-01bd-452f-af72-a5f96285ed69");
    let mut e = lk::game(SEED, &[index::HILL_GIANT], &[subject], &[], &[]);
    let giant = lk::obj(&e, lk::P0, index::HILL_GIANT);
    let wall = lk::obj(&e, lk::P1, subject);
    lk::attackers_offered(&mut e);
    let blockers = lk::declare(&mut e, &[giant]);
    assert!(lk::may_block(&blockers, wall, giant));
    lk::block(&mut e, &[(wall, giant)]);
    lk::through_combat(&mut e);
    assert_eq!(lk::damage_on(&e, wall), 3);
    assert_eq!(lk::life(&e, lk::P0), 20);
    assert_eq!(lk::life(&e, lk::P1), 20);
}
