//! Ruham Djinn — {5}{W} — Creature — Djinn
//! Oracle: First strike
//! Oracle: This creature gets -2/-2 as long as white is the most common color among all permanents or is tied for most common.
//! Set: INV #35 — Invasion | Scryfall ID: a46c7718-1ecc-418c-b213-13be9de5cb7f | Oracle ID: 16ad3b64-a87f-47e8-8e7b-b97bf084a3d2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RUHAM_DJINN,
    oracle_id = "16ad3b64-a87f-47e8-8e7b-b97bf084a3d2",
    scryfall_id = "a46c7718-1ecc-418c-b213-13be9de5cb7f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Ruham Djinn",
        mana_cost = mana!("{5}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
