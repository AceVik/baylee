//! Legion's Landing // Adanto, the First Fort — {W} — Legendary Enchantment // Legendary Land
//! Set: XLN #22 — Ixalan | Scryfall ID: 05e2a5e6-3aaa-4096-bdd0-fcc1afe5a36c | Oracle ID: f7d8b91b-6541-4d3e-af51-7e000eac69c1
//! Face: Legion's Landing — {W} — Legendary Enchantment
//! Face: Adanto, the First Fort —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 710,
    oracle_id: "f7d8b91b-6541-4d3e-af51-7e000eac69c1",
    scryfall_id: "05e2a5e6-3aaa-4096-bdd0-fcc1afe5a36c",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Legion's Landing",
        mana_cost: baylee_core::mana!("{W}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Adanto, the First Fort",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
