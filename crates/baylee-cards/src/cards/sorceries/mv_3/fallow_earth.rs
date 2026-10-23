//! Fallow Earth — {2}{G} — Sorcery
//! Oracle: Put target land on top of its owner's library.
//! Set: 6ED #227 — Classic Sixth Edition | Scryfall ID: 44b02b12-415c-49f4-9f60-fa53beb4216f | Oracle ID: 05a4acfd-3bf3-4ec2-a28a-bc43ba7c6fbf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FALLOW_EARTH,
    oracle_id = "05a4acfd-3bf3-4ec2-a28a-bc43ba7c6fbf",
    scryfall_id = "44b02b12-415c-49f4-9f60-fa53beb4216f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Fallow Earth",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
