//! Krosan Cloudscraper — {7}{G}{G}{G} — Creature — Beast Mutant
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you pay {G}{G}.
//! Oracle: Morph {7}{G}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: MKC #175 — Murders at Karlov Manor Commander | Scryfall ID: c1fab763-4e62-418d-b3a6-77c52bf21f23 | Oracle ID: a0ff742b-f709-43ac-84d2-dc00c723bad6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KROSAN_CLOUDSCRAPER,
    oracle_id = "a0ff742b-f709-43ac-84d2-dc00c723bad6",
    scryfall_id = "c1fab763-4e62-418d-b3a6-77c52bf21f23",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Krosan Cloudscraper",
        mana_cost = mana!("{7}{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST, subtypes::creature::MUTANT],
        power = Some(13),
        toughness = Some(13),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
