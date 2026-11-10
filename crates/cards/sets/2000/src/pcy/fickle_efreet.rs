//! Fickle Efreet — {3}{R} — Creature — Efreet
//! Oracle: Whenever this creature attacks or blocks, flip a coin at end of combat. If you lose the flip, an opponent gains control of this creature.
//! Set: PCY #89 — Prophecy | Scryfall ID: ca6d047a-f3dc-4c34-9679-fb76037e4044 | Oracle ID: e5cd1024-ec1a-4df9-8bc6-ad2f3f443d06
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FICKLE_EFREET,
    oracle_id = "e5cd1024-ec1a-4df9-8bc6-ad2f3f443d06",
    scryfall_id = "ca6d047a-f3dc-4c34-9679-fb76037e4044",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Fickle Efreet",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::EFREET],
        power = Some(5),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
