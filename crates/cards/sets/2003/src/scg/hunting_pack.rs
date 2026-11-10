//! Hunting Pack — {5}{G}{G} — Instant
//! Oracle: Create a 4/4 green Beast creature token.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn.)
//! Set: MH2 #284 — Modern Horizons 2 | Scryfall ID: 8c9eb595-e8fa-4a5e-abca-d30613c0e28f | Oracle ID: d3e1848a-c236-4352-a7be-b0b4699af968
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HUNTING_PACK,
    oracle_id = "d3e1848a-c236-4352-a7be-b0b4699af968",
    scryfall_id = "8c9eb595-e8fa-4a5e-abca-d30613c0e28f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Hunting Pack",
        mana_cost = mana!("{5}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
