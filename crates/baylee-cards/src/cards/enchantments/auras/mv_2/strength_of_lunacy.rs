//! Strength of Lunacy — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/+1 and has protection from white.
//! Oracle: Madness {B} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)
//! Set: TOR #86 — Torment | Scryfall ID: afda26e7-4fdd-45e1-bcd1-f0abf91979aa | Oracle ID: 02a95ea7-cfb9-497c-a004-3e783856d79c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STRENGTH_OF_LUNACY,
    oracle_id = "02a95ea7-cfb9-497c-a004-3e783856d79c",
    scryfall_id = "afda26e7-4fdd-45e1-bcd1-f0abf91979aa",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Strength of Lunacy",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
