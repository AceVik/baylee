//! Spitting Gourna — {3}{G}{G} — Creature — Beast
//! Oracle: Reach (This creature can block creatures with flying.)
//! Oracle: Morph {4}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #284 — Onslaught | Scryfall ID: 746b98bf-5398-4a00-b4fe-a990ea9cfd77 | Oracle ID: 413e8f01-4ee0-48df-8647-c614db38681e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPITTING_GOURNA,
    oracle_id = "413e8f01-4ee0-48df-8647-c614db38681e",
    scryfall_id = "746b98bf-5398-4a00-b4fe-a990ea9cfd77",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Spitting Gourna",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
