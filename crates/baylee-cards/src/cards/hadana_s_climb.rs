//! Hadana's Climb // Winged Temple of Orazca — {1}{G}{U} — Legendary Enchantment // Legendary Land
//! Set: RIX #158 — Rivals of Ixalan | Scryfall ID: 8e7554bc-8583-4059-8895-c3845bc27ae3 | Oracle ID: 93b91d18-6acf-42e5-9a31-bc6e01f90c1f
//! Face: Hadana's Climb — {1}{G}{U} — Legendary Enchantment
//! Face: Winged Temple of Orazca —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 592,
    oracle_id: "93b91d18-6acf-42e5-9a31-bc6e01f90c1f",
    scryfall_id: "8e7554bc-8583-4059-8895-c3845bc27ae3",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces: &[
    face! {
        name: "Hadana's Climb",
        mana_cost: baylee_core::mana!("{1}{G}{U}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Winged Temple of Orazca",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
