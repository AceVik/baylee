//! Reef Shaman — {U} — Creature — Merfolk Shaman
//! Oracle: {T}: Target land becomes the basic land type of your choice until end of turn.
//! Set: APC #29 — Apocalypse | Scryfall ID: c6f56714-0baa-48f9-8da1-50d9279e759c | Oracle ID: 4ad870b2-133c-4229-84a8-f91e4c62fcd0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::REEF_SHAMAN,
    oracle_id = "4ad870b2-133c-4229-84a8-f91e4c62fcd0",
    scryfall_id = "c6f56714-0baa-48f9-8da1-50d9279e759c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Reef Shaman",
        mana_cost = mana!("{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::SHAMAN],
        power = Some(0),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
