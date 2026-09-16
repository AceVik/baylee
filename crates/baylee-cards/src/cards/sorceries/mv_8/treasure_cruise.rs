//! Treasure Cruise — {7}{U} — Sorcery
//! Oracle: Delve (Each card you exile from your graveyard while casting this spell pays for {1}.)
//! Oracle: Draw three cards.
//! Set: SOC #205 — Secrets of Strixhaven Commander | Scryfall ID: 42c45880-15c7-4259-8066-c04d031d8216 | Oracle ID: 5b6bdf5a-2742-4851-92cd-a857a3852836
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TREASURE_CRUISE,
    oracle_id = "5b6bdf5a-2742-4851-92cd-a857a3852836",
    scryfall_id = "42c45880-15c7-4259-8066-c04d031d8216",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Treasure Cruise",
        mana_cost = mana!("{7}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
