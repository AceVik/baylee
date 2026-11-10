//! `cards/artifacts/equipment/mv_3/angelic_armaments.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn angelic_armaments() -> CardIndex {
    card_index("c2042b38-1f06-44d4-b56f-2cb4d9197712")
}

/// Angelic Armaments: "Equipped creature gets +2/+2, has flying, and is a
/// white Angel in addition to its other colors and types. Equip {4}".
#[test]
fn angelic_armaments_makes_the_host_a_white_angel_with_flying() {
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::generated::subtypes::creature::{ANGEL, ELF};
    let mut r = rig(angelic_armaments(), llanowar_elves(), 4);
    assert!(!subtypes_of(&r.engine, r.host).contains(ANGEL));
    assert!(!colors_of(&r.engine, r.host).contains(Color::White));
    r.equip(4);
    assert_eq!(pt(&r.engine, r.host), (3, 3), "+2/+2");
    assert!(keywords(&r.engine, r.host).contains(KeywordSet::FLYING));
    let subtypes = subtypes_of(&r.engine, r.host);
    assert!(subtypes.contains(ANGEL), "an Angel");
    assert!(subtypes.contains(ELF), "in addition to being an Elf");
    let colors = colors_of(&r.engine, r.host);
    assert!(colors.contains(Color::White), "white");
    assert!(
        colors.contains(Color::Green),
        "in addition to its other colors: {colors:?}"
    );
    assert_ne!(colors, ColorSet::of(Color::White));
}
