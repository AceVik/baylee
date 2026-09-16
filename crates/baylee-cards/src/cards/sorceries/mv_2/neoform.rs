//! Neoform — {G}{U} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Search your library for a creature card with mana value equal to 1 plus the sacrificed creature's mana value, put that card onto the battlefield with an additional +1/+1 counter on it, then shuffle.
//! Set: WAR #206 — War of the Spark | Scryfall ID: 92d8f67e-4f2f-4a1f-b190-7c3f39e477e4 | Oracle ID: 420c6dcf-966d-4a4c-a0ef-23037ab8b325
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NEOFORM,
    oracle_id = "420c6dcf-966d-4a4c-a0ef-23037ab8b325",
    scryfall_id = "92d8f67e-4f2f-4a1f-b190-7c3f39e477e4",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Neoform",
        mana_cost = mana!("{G}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
