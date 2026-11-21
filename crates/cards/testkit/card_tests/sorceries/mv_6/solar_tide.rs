//! `cards/sorceries/mv_6/solar_tide.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn solar_tide() -> CardIndex {
    card_index("0f7a7348-3081-4fbb-9386-719a4ae7fa57")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Solar Tide (0f7a7348-3081-4fbb-9386-719a4ae7fa57) — {4}{W}{W} Sorcery:
/// Mode 0: Destroy all creatures with power <= 2; Mode 1: power >= 3;
/// Entwine—Sacrifice two lands.
#[test]
fn solar_tide_destroys_small_or_large_creatures_or_both_when_entwined() {
    let mut m0 = lk::game(
        SEED,
        &[plains(); 6],
        &[aurochs(), hill_giant()],
        &[solar_tide()],
        &[],
    );
    let p2 = lk::obj(&m0, P1, aurochs());
    let p3 = lk::obj(&m0, P1, hill_giant());
    tap_all_mana(&mut m0, P0);
    cast_in_mode(&mut m0, solar_tide(), 0);
    lk::settle(&mut m0);
    assert_eq!(lk::zone_of(&m0, p2), Zone::Graveyard);
    assert_eq!(lk::zone_of(&m0, p3), Zone::Battlefield);

    let mut m1 = lk::game(
        SEED,
        &[plains(); 6],
        &[aurochs(), hill_giant()],
        &[solar_tide()],
        &[],
    );
    let p2_b = lk::obj(&m1, P1, aurochs());
    let p3_b = lk::obj(&m1, P1, hill_giant());
    tap_all_mana(&mut m1, P0);
    cast_in_mode(&mut m1, solar_tide(), 1);
    lk::settle(&mut m1);
    assert_eq!(lk::zone_of(&m1, p2_b), Zone::Battlefield);
    assert_eq!(lk::zone_of(&m1, p3_b), Zone::Graveyard);

    let mut entwined = lk::game(
        SEED,
        &[plains(); 6],
        &[aurochs(), hill_giant()],
        &[solar_tide()],
        &[],
    );
    let small = lk::obj(&entwined, P1, aurochs());
    let big = lk::obj(&entwined, P1, hill_giant());
    tap_all_mana(&mut entwined, P0);
    cast_with_floating(&mut entwined, P0, solar_tide());
    let slot = choose_cast_kind(&entwined, CastModeKind::Modes(3));
    entwined.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
    for _ in 0..2 {
        let Pending::ChooseCards { options, .. } = entwined.pending().clone() else {
            panic!("expected land sacrifice, got {:?}", entwined.pending());
        };
        entwined
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    lk::settle(&mut entwined);
    assert_eq!(lk::zone_of(&entwined, small), Zone::Graveyard);
    assert_eq!(lk::zone_of(&entwined, big), Zone::Graveyard);
    assert_eq!(
        entwined
            .state()
            .zones
            .list(ZoneLocation::Graveyard(P0))
            .len(),
        3,
        "2 sacrificed Plains + Solar Tide"
    );
}
