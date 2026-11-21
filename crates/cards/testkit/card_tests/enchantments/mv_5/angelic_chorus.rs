//! `cards/enchantments/mv_5/angelic_chorus.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn angelic_chorus() -> CardIndex {
    card_index("e66b12ff-45da-46dc-9148-bd8ea9b7fde0")
}

/// Whenever a creature you control enters, you gain life equal to its
/// toughness (asymmetric 3/5 proves toughness rather than power; opponent's
/// creature entering grants no life).
#[test]
fn angelic_chorus_gains_life_equal_to_entering_your_creatures_toughness() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[angelic_chorus()],
        &[],
        &[killer_whale()],
        &[grizzly_bears()],
    );

    lk::float(&mut e, p0, &[(ManaColor::Blue, 5)]);
    cast_with_floating(&mut e, p0, killer_whale());
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p0), 25, "3/5 Killer Whale has toughness 5");

    // Opponent casting Grizzly Bears does not gain life for either player.
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, p1, grizzly_bears());
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p0), 25);
    assert_eq!(lk::life(&e, p1), 20);
}
