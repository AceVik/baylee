//! Magnigoth Treefolk — {4}{G} — Creature — Treefolk
//! Oracle: Domain — For each basic land type among lands you control, this creature has landwalk of that type. (It can't be blocked as long as defending player controls a land of that type.)
//! Set: PLS #82 — Planeshift | Scryfall ID: 90c2869b-43cf-4d5e-8a54-9ae200f5bff9 | Oracle ID: b153e4c4-6557-43a6-8404-e4450f879945
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MAGNIGOTH_TREEFOLK,
    oracle_id = "b153e4c4-6557-43a6-8404-e4450f879945",
    scryfall_id = "90c2869b-43cf-4d5e-8a54-9ae200f5bff9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Magnigoth Treefolk",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(2),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
