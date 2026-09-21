//! Skyline Cascade — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, target creature an opponent controls doesn't untap during its controller's next untap step.
//! Oracle: {T}: Add {U}.
//! Set: BFZ #246 — Battle for Zendikar | Scryfall ID: 29b0027d-c232-4cdd-89c4-75947687aa71 | Oracle ID: 79301ae1-8c9c-4723-be21-dc27e1646f35
// PARTIAL — enters tapped and {T}: Add {U} are built; duration-based DoesNotUntap for next untap step has no DSL variant.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SKYLINE_CASCADE,
    oracle_id = "79301ae1-8c9c-4723-be21-dc27e1646f35",
    scryfall_id = "29b0027d-c232-4cdd-89c4-75947687aa71",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Skyline Cascade",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "keeping target creature from untapping during its controller's next untap step is not expressible in the DSL"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        // NOT SUPPORTED: "When this land enters, target creature an opponent controls doesn't untap during its controller's next untap step."
    ],
);
