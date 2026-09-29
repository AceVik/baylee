//! Evil Presence — {B} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land is a Swamp.
//! Set: NPH #60 — New Phyrexia | Scryfall ID: 4dd00bc5-234c-4ded-b0b7-1181bc16cb28 | Oracle ID: 3d8ac41c-0566-48b2-a744-39db2f72272c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EVIL_PRESENCE,
    oracle_id = "3d8ac41c-0566-48b2-a744-39db2f72272c",
    scryfall_id = "4dd00bc5-234c-4ded-b0b7-1181bc16cb28",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Evil Presence",
        mana_cost = mana!("{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
