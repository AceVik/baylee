//! Thalakos Mistfolk — {2}{U} — Creature — Thalakos Illusion
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: {U}: Put this creature on top of its owner's library.
//! Set: TMP #93 — Tempest | Scryfall ID: 9e7b5b00-9d14-4090-b8c3-28b70375571e | Oracle ID: c836f8e4-6154-4c7a-a8dd-165ff74f9115

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THALAKOS_MISTFOLK,
    oracle_id = "c836f8e4-6154-4c7a-a8dd-165ff74f9115",
    scryfall_id = "9e7b5b00-9d14-4090-b8c3-28b70375571e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::SHADOW,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thalakos Mistfolk",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::THALAKOS, subtypes::creature::ILLUSION],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[activated!(cost!("{U}"), &[Effect::PutSourceOnTopOfLibrary])],
);
