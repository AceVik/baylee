//! Gurzigost — {3}{G}{G} — Creature — Beast
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you put two cards from your graveyard on the bottom of your library.
//! Oracle: {G}{G}, Discard a card: You may have this creature assign its combat damage this turn as though it weren't blocked.
//! Set: TOR #126 — Torment | Scryfall ID: f4e672c6-6ddc-4dd2-b4c7-5083d7566e87 | Oracle ID: 835733a2-3738-4fc0-97d5-33156b711ba4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GURZIGOST,
    oracle_id = "835733a2-3738-4fc0-97d5-33156b711ba4",
    scryfall_id = "f4e672c6-6ddc-4dd2-b4c7-5083d7566e87",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Gurzigost",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(6),
        toughness = Some(8),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
