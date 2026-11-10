//! Cleansing Meditation — {1}{W}{W} — Sorcery
//! Oracle: Destroy all enchantments.
//! Oracle: Threshold — If there are seven or more cards in your graveyard, instead destroy all enchantments, then return all cards in your graveyard destroyed this way to the battlefield.
//! Set: TOR #3 — Torment | Scryfall ID: fd6609ef-71af-4775-affc-34153700c556 | Oracle ID: 658bccf8-fe73-4d6a-b37b-7a58034e5e5d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CLEANSING_MEDITATION,
    oracle_id = "658bccf8-fe73-4d6a-b37b-7a58034e5e5d",
    scryfall_id = "fd6609ef-71af-4775-affc-34153700c556",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Cleansing Meditation",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
