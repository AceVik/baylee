//! Journey to Eternity // Atzal, Cave of Eternity — {1}{B}{G} — Legendary Enchantment — Aura // Legendary Land
//! Set: RIX #160 — Rivals of Ixalan | Scryfall ID: d81c4b3f-81c2-403b-8a5d-c9415f73a1f9 | Oracle ID: 7d6ccd0b-df16-40b2-930b-bcde0b6ef73f
//! Face: Journey to Eternity — {1}{B}{G} — Legendary Enchantment — Aura
//! Face: Atzal, Cave of Eternity —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 668,
    oracle_id: "7d6ccd0b-df16-40b2-930b-bcde0b6ef73f",
    scryfall_id: "d81c4b3f-81c2-403b-8a5d-c9415f73a1f9",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces: &[
    face! {
        name: "Journey to Eternity",
        mana_cost: baylee_core::mana!("{1}{B}{G}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::enchantment::AURA],
    },
    face! {
        name: "Atzal, Cave of Eternity",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
