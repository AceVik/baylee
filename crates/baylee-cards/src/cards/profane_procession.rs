//! Profane Procession // Tomb of the Dusk Rose — {1}{W}{B} — Legendary Enchantment // Legendary Land
//! Set: RIX #166 — Rivals of Ixalan | Scryfall ID: 1d94ff37-f04e-48ee-8253-d62ab07f0632 | Oracle ID: a656ad7f-133f-4d93-919a-43bcf1f815f3
//! Face: Profane Procession — {1}{W}{B} — Legendary Enchantment
//! Face: Tomb of the Dusk Rose —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 878,
    oracle_id: "a656ad7f-133f-4d93-919a-43bcf1f815f3",
    scryfall_id: "1d94ff37-f04e-48ee-8253-d62ab07f0632",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    faces: &[
    face! {
        name: "Profane Procession",
        mana_cost: baylee_core::mana!("{1}{W}{B}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Tomb of the Dusk Rose",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
