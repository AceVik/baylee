//! Planar Nexus — (no cost) — Land
//! Oracle: This land is every nonbasic land type. (Nonbasic land types include Cave, Desert, Gate, Lair, Locus, Mine, Power-Plant, Sphere, Tower, and Urza's.)
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Set: M3C #80 — Modern Horizons 3 Commander | Scryfall ID: 28603c1c-f9b4-4001-bc56-d1453d5cacf5 | Oracle ID: 26005003-afcb-4c32-a760-be950246ff0f

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANAR_NEXUS,
    oracle_id = "26005003-afcb-4c32-a760-be950246ff0f",
    scryfall_id = "28603c1c-f9b4-4001-bc56-d1453d5cacf5",
    faces = &[face!(name = "Planar Nexus", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    keywords = KeywordSet::ALL_NONBASIC_LAND_TYPES,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
    ],
);
