//! Tribal Flames — {1}{R} — Sorcery
//! Oracle: Domain — Tribal Flames deals X damage to any target, where X is the number of basic land types among lands you control.
//! Set: MM2 #132 — Modern Masters 2015 | Scryfall ID: d22418cd-4c49-4754-aa75-17f6eaf1639a | Oracle ID: c82eda61-d195-49c9-8ff0-a78dfa650689
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRIBAL_FLAMES,
    oracle_id = "c82eda61-d195-49c9-8ff0-a78dfa650689",
    scryfall_id = "d22418cd-4c49-4754-aa75-17f6eaf1639a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Tribal Flames",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
