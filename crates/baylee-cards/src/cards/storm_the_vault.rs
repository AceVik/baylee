//! Storm the Vault // Vault of Catlacan — {2}{U}{R} — Legendary Enchantment // Legendary Land
//! Set: RIX #173 — Rivals of Ixalan | Scryfall ID: c16ba84e-a0cc-4c6c-9b80-713247b8fef9 | Oracle ID: 72205fac-a94a-45cc-94c6-40ece2fdce0e
//! Face: Storm the Vault — {2}{U}{R} — Legendary Enchantment
//! Face: Vault of Catlacan —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1091,
    oracle_id: "72205fac-a94a-45cc-94c6-40ece2fdce0e",
    scryfall_id: "c16ba84e-a0cc-4c6c-9b80-713247b8fef9",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces: &[
    face! {
        name: "Storm the Vault",
        mana_cost: baylee_core::mana!("{2}{U}{R}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Vault of Catlacan",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
