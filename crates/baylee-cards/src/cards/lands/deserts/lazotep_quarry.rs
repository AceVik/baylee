//! Lazotep Quarry — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice a creature: Add one mana of any color.
//! Oracle: {X}{2}, {T}, Sacrifice a Desert: Exile target creature card with mana value X from your graveyard. Create a token that's a copy of it, except it's a 4/4 black Zombie. Activate only as a sorcery.
//! Set: M3C #79 — Modern Horizons 3 Commander | Scryfall ID: ff73b7f3-62f3-4a05-b439-bae2d0f63d2f | Oracle ID: 0d2fa39a-9cac-4a1f-bb1e-b6162e6d5169
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 707,
    oracle_id: "0d2fa39a-9cac-4a1f-bb1e-b6162e6d5169",
    scryfall_id: "ff73b7f3-62f3-4a05-b439-bae2d0f63d2f",
    faces: &[
    face! {
        name: "Lazotep Quarry",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
