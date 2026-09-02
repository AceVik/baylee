//! Arguel's Blood Fast // Temple of Aclazotz — {1}{B} — Legendary Enchantment // Legendary Land
//! Set: XLN #90 — Ixalan | Scryfall ID: c4ac7570-e74e-4081-ac53-cf41e695b7eb | Oracle ID: be2a4bc4-8af6-48c5-9421-32d26272e71a
//! Face: Arguel's Blood Fast — {1}{B} — Legendary Enchantment
//! Face: Temple of Aclazotz —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 237,
    oracle_id: "be2a4bc4-8af6-48c5-9421-32d26272e71a",
    scryfall_id: "c4ac7570-e74e-4081-ac53-cf41e695b7eb",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Arguel's Blood Fast",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Temple of Aclazotz",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
