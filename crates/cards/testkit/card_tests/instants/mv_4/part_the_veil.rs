//! `cards/instants/mv_4/part_the_veil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "c8ca2523-0511-4415-b85e-7834d3cb1d0c"
fn part_the_veil() -> CardIndex {
    card_index("c8ca2523-0511-4415-b85e-7834d3cb1d0c")
}

/// Part the Veil ({3}{U}): returns all creatures you control to their OWNER's
/// hand (so a stolen opponent's creature returns to the opponent's hand),
/// leaving noncreatures and creatures you don't control on the battlefield.
#[test]
fn part_the_veil_returns_controlled_creatures_to_owners_hands() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, sol_ring()],
        &[index::HILL_GIANT, index::SAVANNAH_LIONS],
        &[control_magic(), part_the_veil()],
        &[],
    );
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let lions = lk::obj(&engine, P1, index::SAVANNAH_LIONS);
    let ring = lk::obj(&engine, P0, sol_ring());

    // Steal P1's Hill Giant with Control Magic so P0 controls it while P1 owns it.
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 4)]);
    lk::cast_at(&mut engine, P0, control_magic(), &[giant], &[]);
    assert_eq!(on_battlefield(&engine, P0, index::HILL_GIANT), Some(giant));

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 4)]);
    cast_with_floating(&mut engine, P0, part_the_veil());
    lk::settle(&mut engine);

    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(
        in_hand(&engine, P1, index::HILL_GIANT).is_some(),
        "stolen Hill Giant returned to its OWNER's (P1's) hand"
    );
    assert_eq!(on_battlefield(&engine, P0, sol_ring()), Some(ring));
    assert_eq!(
        on_battlefield(&engine, P1, index::SAVANNAH_LIONS),
        Some(lions)
    );
}
