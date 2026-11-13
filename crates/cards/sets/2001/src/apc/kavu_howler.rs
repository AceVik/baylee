//! Kavu Howler — {4}{G}{G} — Creature — Kavu
//! Oracle: When this creature enters, reveal the top four cards of your library. Put all Kavu cards revealed this way into your hand and the rest on the bottom of your library in any order.
//! Set: APC #79 — Apocalypse | Scryfall ID: 5fdf502f-445d-4724-b7d0-8fdd5bf557a8 | Oracle ID: 635c0a7d-d832-4c10-a5f0-9cde19152cbb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KAVU_HOWLER,
    oracle_id = "635c0a7d-d832-4c10-a5f0-9cde19152cbb",
    scryfall_id = "5fdf502f-445d-4724-b7d0-8fdd5bf557a8",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Kavu Howler",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(4),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
