//! Ageless Sentinels — {3}{W} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Flying
//! Oracle: When this creature blocks, it becomes a Bird Giant, and it loses defender. (It's no longer a Wall. This effect lasts indefinitely.)
//! Set: SCG #1 — Scourge | Scryfall ID: ccaa4a19-8eba-4c43-8a9a-636e234df751 | Oracle ID: 9023c530-f329-4301-8b24-4cf676194265
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AGELESS_SENTINELS,
    oracle_id = "9023c530-f329-4301-8b24-4cf676194265",
    scryfall_id = "ccaa4a19-8eba-4c43-8a9a-636e234df751",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Ageless Sentinels",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
