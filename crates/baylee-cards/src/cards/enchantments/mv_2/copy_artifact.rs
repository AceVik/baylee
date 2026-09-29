//! Copy Artifact — {1}{U} — Enchantment
//! Oracle: You may have this enchantment enter as a copy of any artifact on the battlefield, except it's an enchantment in addition to its other types.
//! Set: ME4 #44 — Masters Edition IV | Scryfall ID: cd360a6c-6685-462b-8aee-b869c1e4aa89 | Oracle ID: 80bc56a9-40e0-48da-ae86-190e39c8a4a3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COPY_ARTIFACT,
    oracle_id = "80bc56a9-40e0-48da-ae86-190e39c8a4a3",
    scryfall_id = "cd360a6c-6685-462b-8aee-b869c1e4aa89",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Copy Artifact",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
