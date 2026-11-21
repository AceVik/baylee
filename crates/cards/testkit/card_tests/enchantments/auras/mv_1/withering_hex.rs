//! `cards/enchantments/auras/mv_1/withering_hex.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn withering_hex() -> CardIndex {
    card_index("96e41850-04a9-47bf-98a7-4238b25b1824")
}

fn horned_turtle() -> CardIndex {
    card_index("7a4ae76b-6241-4fa8-b239-5ec941a0e7df")
}

fn secluded_steppe() -> CardIndex {
    card_index("8dec6fcf-1254-4b1b-ba23-7a3e492a7241")
}

/// Cycling by either player adds a plague counter, giving the enchanted creature
/// -1/-1 per counter until lethal toughness kills it.
#[test]
fn withering_hex_grows_plague_counters_when_either_player_cycles() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[plains()],
        &[horned_turtle(), llanowar_elves(), plains()],
        &[withering_hex(), secluded_steppe(), secluded_steppe()],
        &[secluded_steppe(), secluded_steppe()],
    );
    let turtle = lk::obj(&e, p1, horned_turtle());
    let elves = lk::obj(&e, p1, llanowar_elves());

    lk::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    lk::cast_at(&mut e, p0, withering_hex(), &[turtle], &[]);
    assert_eq!(pt(&e, turtle), (1, 4));

    // P0 cycles twice -> 2 plague counters -> Turtle becomes -1/2; Elves stay 1/1.
    for _ in 0..2 {
        lk::float(&mut e, p0, &[(ManaColor::White, 1)]);
        activate(&mut e, p0, secluded_steppe(), 1);
        lk::settle(&mut e);
    }
    assert_eq!(pt(&e, turtle), (-1, 2));
    assert_eq!(pt(&e, elves), (1, 1));

    // P1 cycles twice on P1's main phase -> 4 plague counters -> Turtle (1/4) dies.
    reach_their_main_phase(&mut e, p1);
    for _ in 0..2 {
        lk::float(&mut e, p1, &[(ManaColor::White, 1)]);
        activate(&mut e, p1, secluded_steppe(), 1);
        lk::settle(&mut e);
    }
    assert_eq!(lk::zone_of(&e, turtle), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, elves), Zone::Battlefield);
}
