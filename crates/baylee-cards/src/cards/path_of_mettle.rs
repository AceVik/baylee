//! Path of Mettle // Metzali, Tower of Triumph — {R}{W} — Legendary Enchantment // Legendary Land
//! Set: RIX #165 — Rivals of Ixalan | Scryfall ID: 66d9d524-3611-48d9-86c9-48e509e8ae70 | Oracle ID: db9ea3f9-c723-422f-98cc-a3ef7ca2c290
//! Face: Path of Mettle — {R}{W} — Legendary Enchantment
//! Face: Metzali, Tower of Triumph —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 849,
    oracle_id: "db9ea3f9-c723-422f-98cc-a3ef7ca2c290",
    scryfall_id: "66d9d524-3611-48d9-86c9-48e509e8ae70",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::White]),
    faces: &[
    face! {
        name: "Path of Mettle",
        mana_cost: baylee_core::mana!("{R}{W}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Metzali, Tower of Triumph",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
