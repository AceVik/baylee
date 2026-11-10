//! Karona's Zealot — {4}{W} — Creature — Human Cleric
//! Oracle: Morph {3}{W}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, all damage that would be dealt to it this turn is dealt to target creature instead.
//! Set: A25 #18 — Masters 25 | Scryfall ID: 29214308-711a-490f-8e93-bf61baf1749c | Oracle ID: dff66bcf-e126-49d2-b67e-ea3b0a38d390
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KARONA_S_ZEALOT,
    oracle_id = "dff66bcf-e126-49d2-b67e-ea3b0a38d390",
    scryfall_id = "29214308-711a-490f-8e93-bf61baf1749c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Karona's Zealot",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
