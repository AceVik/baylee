//! Rite of Flame — {R} — Sorcery
//! Oracle: Add {R}{R}, then add {R} for each card named Rite of Flame in each graveyard.
//! Set: CSP #96 — Coldsnap | Scryfall ID: c062caf7-f0eb-44db-9f74-e6711a13fada | Oracle ID: 8a2e53f9-8100-488f-8504-b59e9bd1cc29
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RITE_OF_FLAME,
    oracle_id = "8a2e53f9-8100-488f-8504-b59e9bd1cc29",
    scryfall_id = "c062caf7-f0eb-44db-9f74-e6711a13fada",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Rite of Flame",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
