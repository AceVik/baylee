//! Animal Boneyard — {2}{W} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "{T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness."
//! Set: ODY #4 — Odyssey | Scryfall ID: e3379317-be18-4252-84ad-b7ebb6e557ff | Oracle ID: 6fa9debf-c60e-432d-86c8-aed16556c532
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANIMAL_BONEYARD,
    oracle_id = "6fa9debf-c60e-432d-86c8-aed16556c532",
    scryfall_id = "e3379317-be18-4252-84ad-b7ebb6e557ff",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Animal Boneyard",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
