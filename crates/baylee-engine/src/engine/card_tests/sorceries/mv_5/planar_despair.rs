//! Oracle scenarios for Planar Despair.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn planar_despair() -> CardIndex {
    card_index("1ba36042-8902-4abb-a4ae-a8d26d81a0de")
}

#[test]
fn planar_despair_shrinks_all_creatures_by_domain_count() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(
            0,
            &[
                index::SWAMP,
                index::SWAMP,
                index::ISLAND,
                index::MOUNTAIN,
                index::PLAINS,
                index::EARTH_ELEMENTAL,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT, index::MAHAMOTI_DJINN])
        .hand(0, &[planar_despair()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let elemental = on_battlefield(&e, p0, index::EARTH_ELEMENTAL).unwrap();
    let djinn = on_battlefield(&e, p1, index::MAHAMOTI_DJINN).unwrap();

    // Domain = 4 (Swamp, Island, Mountain, Plains) -> all creatures get -4/-4 until end of turn.
    cast_from_hand(&mut e, p0, planar_despair());
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, p1, index::HILL_GIANT).is_some(),
        "3/3 Hill Giant died to -4/-4"
    );
    assert_eq!(pt(&e, elemental), (0, 1), "4/5 Earth Elemental became 0/1");
    assert_eq!(pt(&e, djinn), (1, 2), "5/6 Mahamoti Djinn became 1/2");

    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, elemental), (4, 5));
    assert_eq!(pt(&e, djinn), (5, 6));
}
