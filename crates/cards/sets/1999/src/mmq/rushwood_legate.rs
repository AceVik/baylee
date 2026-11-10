//! Rushwood Legate — {2}{G} — Creature — Dryad
//! Oracle: If an opponent controls an Island and you control a Forest, you may cast this spell without paying its mana cost.
//! Set: MMQ #266 — Mercadian Masques | Scryfall ID: 827b9c99-87d7-493c-9dc3-0c6aa4a61b49 | Oracle ID: f3623759-bf3a-4846-9ef7-1b732ad4355f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RUSHWOOD_LEGATE,
    oracle_id = "f3623759-bf3a-4846-9ef7-1b732ad4355f",
    scryfall_id = "827b9c99-87d7-493c-9dc3-0c6aa4a61b49",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Rushwood Legate",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRYAD],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
