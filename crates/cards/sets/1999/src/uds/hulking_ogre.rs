//! Hulking Ogre — {2}{R} — Creature — Ogre
//! Oracle: This creature can't block.
//! Set: S99 #108 — Starter 1999 | Scryfall ID: 7acfffab-0532-438d-978c-9fd986420ee6 | Oracle ID: a95aa9ba-fa61-4a3d-a493-1ea790bd20ad
// IMPLEMENTED — 3/3 Ogre that can't block.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HULKING_OGRE,
    oracle_id = "a95aa9ba-fa61-4a3d-a493-1ea790bd20ad",
    scryfall_id = "7acfffab-0532-438d-978c-9fd986420ee6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::CANT_BLOCK,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hulking Ogre",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OGRE],
        power = Some(3),
        toughness = Some(3),
    ),],
);
