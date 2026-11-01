//! `cards/enchantments/mv_2/tranquil_grove.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index::{BAD_MOON, CRUSADE, GRIZZLY_BEARS, SOL_RING};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tranquil_grove() -> CardIndex {
    card_index("2f2a3588-b9a6-4c1a-89f8-b11339851b8f")
}

/// Tranquil Grove — {1}{G}{G} destroys every OTHER enchantment, on either
/// side, while creatures, artifacts, lands and the Grove survive. One green
/// among three mana cannot pay the activation; two greens can.
#[test]
fn tranquil_grove_destroys_only_the_other_enchantments_for_one_green_green() {
    let mut e = lk::game(
        SEED,
        &[tranquil_grove(), CRUSADE, GRIZZLY_BEARS, forest()],
        &[BAD_MOON, SOL_RING],
        &[],
        &[],
    );
    assert!(!lk::offered(&e, tranquil_grove(), P0, 0));
    assert_eq!(tap_all_mana(&mut e, P0), 1);
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 2)]);
    assert!(
        !lk::offered(&e, tranquil_grove(), P0, 0),
        "a second green is required"
    );
    lk::float(&mut e, P0, &[(ManaColor::Green, 1)]);
    assert!(lk::offered(&e, tranquil_grove(), P0, 0));
    activate(&mut e, P0, tranquil_grove(), 0);
    lk::settle(&mut e);
    assert_eq!(lk::pool_total(&e, P0), 1, "three of the four mana spent");
    assert!(on_battlefield(&e, P0, tranquil_grove()).is_some());
    assert!(in_graveyard(&e, P0, CRUSADE).is_some());
    assert!(in_graveyard(&e, P1, BAD_MOON).is_some());
    assert!(on_battlefield(&e, P0, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P0, forest()).is_some());
    assert!(on_battlefield(&e, P1, SOL_RING).is_some());
}
