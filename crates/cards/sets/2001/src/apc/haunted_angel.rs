//! Haunted Angel — {2}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: When this creature dies, exile it and each other player creates a 3/3 black Angel creature token with flying.
//! Set: APC #12 — Apocalypse | Scryfall ID: 78d2d11b-12e4-4810-a32d-8f1cdda3ec49 | Oracle ID: 437be0ad-2dd1-42c8-bf62-5af95228b2f8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HAUNTED_ANGEL,
    oracle_id = "437be0ad-2dd1-42c8-bf62-5af95228b2f8",
    scryfall_id = "78d2d11b-12e4-4810-a32d-8f1cdda3ec49",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Haunted Angel",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
