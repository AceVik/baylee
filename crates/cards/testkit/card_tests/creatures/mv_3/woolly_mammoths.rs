#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;
use baylee_core::generated::index::{SNOW_COVERED_FOREST, STONE_RAIN};

fn subject() -> CardIndex {
    card_index("6e5f1088-a5ab-48ec-be27-c88eb164cf63")
}

// Scenario: opponent snow land and own ordinary land do not grant trample;
// playing our snow land turns it on without changing the Mammoths' 3/2 body.
#[test]
fn woolly_mammoths_trample_only_with_our_snow_land() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[SNOW_COVERED_FOREST])
        .hand(0, &[subject(), SNOW_COVERED_FOREST, STONE_RAIN])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    let mammoth = on_battlefield(&e, p0, subject()).unwrap();
    assert_eq!(pt(&e, mammoth), (3, 2));
    assert!(!keywords(&e, mammoth).contains(KeywordSet::TRAMPLE));
    let land = in_hand(&e, p0, SNOW_COVERED_FOREST).unwrap();
    e.apply(p0, PlayerAction::PlayLand { card: land }).unwrap();
    assert!(on_battlefield(&e, p0, SNOW_COVERED_FOREST).is_some());
    assert_eq!(pt(&e, mammoth), (3, 2));
    assert!(keywords(&e, mammoth).contains(KeywordSet::TRAMPLE));
    let snow = on_battlefield(&e, p0, SNOW_COVERED_FOREST).unwrap();
    reach_their_main_phase(&mut e, PlayerId::new(1));
    assert!(walk_to_own_main(&mut e, p0));
    cast_from_hand(&mut e, p0, STONE_RAIN);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![snow],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, SNOW_COVERED_FOREST).is_none());
    assert!(on_battlefield(&e, PlayerId::new(1), SNOW_COVERED_FOREST).is_some());
    assert!(!keywords(&e, mammoth).contains(KeywordSet::TRAMPLE));
}
