//! Lazotep Quarry — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice a creature: Add one mana of any color.
//! Oracle: {X}{2}, {T}, Sacrifice a Desert: Exile target creature card with mana value X from your graveyard. Create a token that's a copy of it, except it's a 4/4 black Zombie. Activate only as a sorcery.
//! Set: PLST #M3C-131 — The List | Scryfall ID: 22bf056b-24bb-4f32-93af-088817f42ce8 | Oracle ID: 0d2fa39a-9cac-4a1f-bb1e-b6162e6d5169
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LAZOTEP_QUARRY,
    oracle_id = "0d2fa39a-9cac-4a1f-bb1e-b6162e6d5169",
    scryfall_id = "22bf056b-24bb-4f32-93af-088817f42ce8",
    faces = &[face!(
        name = "Lazotep Quarry",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
