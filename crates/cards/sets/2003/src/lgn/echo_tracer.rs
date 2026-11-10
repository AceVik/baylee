//! Echo Tracer — {2}{U} — Creature — Human Wizard
//! Oracle: Morph {2}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, return target creature to its owner's hand.
//! Set: DDN #51 — Duel Decks: Speed vs. Cunning | Scryfall ID: e856fbd5-0e8b-4f28-bfea-0ad73418d140 | Oracle ID: 34efa2fe-95ef-4a06-9a84-97f03ec68908
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ECHO_TRACER,
    oracle_id = "34efa2fe-95ef-4a06-9a84-97f03ec68908",
    scryfall_id = "e856fbd5-0e8b-4f28-bfea-0ad73418d140",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Echo Tracer",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
