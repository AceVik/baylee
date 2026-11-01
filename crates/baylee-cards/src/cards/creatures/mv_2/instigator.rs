//! Instigator — {1}{B} — Creature — Human Spellshaper
//! Oracle: {1}{B}{B}, {T}, Discard a card: Creatures target player controls attack this turn if able.
//! Set: MMQ #140 — Mercadian Masques | Scryfall ID: 2e3f57af-17d4-4a4c-ae46-fe37f97466fa | Oracle ID: 0fe1a1f7-41c9-415e-a5e3-b5ba0c0271e5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INSTIGATOR,
    oracle_id = "0fe1a1f7-41c9-415e-a5e3-b5ba0c0271e5",
    scryfall_id = "2e3f57af-17d4-4a4c-ae46-fe37f97466fa",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Instigator",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SPELLSHAPER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
