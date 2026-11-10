//! Exotic Curse — {2}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Domain — Enchanted creature gets -1/-1 for each basic land type among lands you control.
//! Set: DDE #56 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: 1eff81c4-c402-437d-aace-dfe875d9857d | Oracle ID: 344476b1-8710-4c7c-826d-efb7d13ceaa7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EXOTIC_CURSE,
    oracle_id = "344476b1-8710-4c7c-826d-efb7d13ceaa7",
    scryfall_id = "1eff81c4-c402-437d-aace-dfe875d9857d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Exotic Curse",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
