//! Strength of Isolation — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+2 and has protection from black.
//! Oracle: Madness {W} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)
//! Set: TOR #17 — Torment | Scryfall ID: 7676e6fb-030d-40aa-a914-27ab6d7d5fc5 | Oracle ID: 87aad9fb-e234-4171-a4ae-c686f613d37e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STRENGTH_OF_ISOLATION,
    oracle_id = "87aad9fb-e234-4171-a4ae-c686f613d37e",
    scryfall_id = "7676e6fb-030d-40aa-a914-27ab6d7d5fc5",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Strength of Isolation",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
