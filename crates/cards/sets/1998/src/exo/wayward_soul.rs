//! Wayward Soul — {2}{U}{U} — Creature — Spirit
//! Oracle: Flying
//! Oracle: {U}: Put this creature on top of its owner's library.
//! Set: TPR #78 — Tempest Remastered | Scryfall ID: 4a761397-3613-4d91-adbf-978a12e7ef32 | Oracle ID: 319b4715-c3eb-49bf-b8d2-4d41e601ebe4
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WAYWARD_SOUL,
    oracle_id = "319b4715-c3eb-49bf-b8d2-4d41e601ebe4",
    scryfall_id = "4a761397-3613-4d91-adbf-978a12e7ef32",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wayward Soul",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(3),
        toughness = Some(2),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[activated!(cost!("{U}"), &[Effect::PutSourceOnTopOfLibrary])],
);
