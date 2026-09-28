//! Natural Order — {2}{G}{G} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a green creature.
//! Oracle: Search your library for a green creature card, put it onto the battlefield, then shuffle.
//! Set: EMA #177 — Eternal Masters | Scryfall ID: bfe3329c-7faa-4925-b9d2-075a1ab27e80 | Oracle ID: 8c1fe337-375a-4add-93b6-0ac39ed72b4f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NATURAL_ORDER,
    oracle_id = "8c1fe337-375a-4add-93b6-0ac39ed72b4f",
    scryfall_id = "bfe3329c-7faa-4925-b9d2-075a1ab27e80",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Natural Order",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
