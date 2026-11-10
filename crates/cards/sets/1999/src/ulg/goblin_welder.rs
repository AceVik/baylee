//! Goblin Welder — {R} — Creature — Goblin Artificer
//! Oracle: {T}: Choose target artifact a player controls and target artifact card in that player's graveyard. If both targets are still legal as this ability resolves, that player simultaneously sacrifices the artifact and returns the artifact card to the battlefield.
//! Set: CM2 #101 — Commander Anthology Volume II | Scryfall ID: 17cd920a-de09-454a-a9da-c84512e3aff1 | Oracle ID: 9fc28e1f-f4ea-4bf8-9442-a36dba3b3b29
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_WELDER,
    oracle_id = "9fc28e1f-f4ea-4bf8-9442-a36dba3b3b29",
    scryfall_id = "17cd920a-de09-454a-a9da-c84512e3aff1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Welder",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::ARTIFICER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
