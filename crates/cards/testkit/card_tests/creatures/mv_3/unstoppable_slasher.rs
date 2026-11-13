//! `cards/creatures/mv_3/unstoppable_slasher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn slasher() -> CardIndex {
    card_index("6662968e-ca9e-454c-be44-24ad10e43d3e")
}

#[test]
fn slasher_combat_damage_halves_the_damaged_players_remaining_life_rounded_up() {
    let mut e = lk::game(SEED, &[slasher()], &[], &[], &[]);
    e.dev_state_mut(P0).unwrap().players[1].life = 15;
    let attacker = lk::obj(&e, P0, slasher());
    lk::attack(&mut e, &[attacker]);
    lk::through_combat(&mut e);

    // Two combat damage leaves 13; losing half of 13, rounded up, loses 7.
    assert_eq!(lk::life(&e, P1), 6);
    assert_eq!(lk::life(&e, P0), 20);
}

#[test]
fn slasher_kills_a_larger_blocker_with_deathtouch_and_returns_without_its_damage() {
    let mut e = lk::game(SEED, &[slasher()], &[rootbreaker_wurm()], &[], &[]);
    let attacker = lk::obj(&e, P0, slasher());
    let blocker = lk::obj(&e, P1, rootbreaker_wurm());
    assert_eq!(pt(&e, blocker), (6, 6));

    let offer = lk::attack(&mut e, &[attacker]);
    assert!(lk::may_block(&offer, blocker, attacker));
    lk::block(&mut e, &[(blocker, attacker)]);
    lk::through_combat(&mut e);

    assert_eq!(lk::zone_of(&e, blocker), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, attacker), Zone::Battlefield);
    assert_eq!(lk::damage_on(&e, attacker), 0, "the returned card is new");
    assert!(lk::tapped(&e, attacker));
    assert_eq!(counters_on(&e, attacker, CounterKind::Stun), 2);
}

#[test]
fn slasher_returns_tapped_with_two_stun_counters_but_not_when_it_had_a_counter() {
    let mut e = lk::game(SEED, &[slasher()], &[], &[], &[]);
    let original = lk::obj(&e, P0, slasher());
    kill(&mut e, original);

    let returned = lk::obj(&e, P0, slasher());
    assert_eq!(returned, original, "the same physical card returns");
    assert!(lk::tapped(&e, returned));
    assert_eq!(counters_on(&e, returned, CounterKind::Stun), 2);
    assert_eq!(lk::zone_of(&e, returned), Zone::Battlefield);

    kill(&mut e, returned);
    assert_eq!(lk::zone_of(&e, returned), Zone::Graveyard);
    assert!(on_battlefield(&e, P0, slasher()).is_none());
}

#[test]
fn slasher_checks_every_counter_kind_as_it_died() {
    let mut e = lk::game(SEED, &[slasher()], &[], &[], &[]);
    let original = lk::obj(&e, P0, slasher());
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(original)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 1);
    kill(&mut e, original);

    assert_eq!(lk::zone_of(&e, original), Zone::Graveyard);
    assert!(on_battlefield(&e, P0, slasher()).is_none());
}
