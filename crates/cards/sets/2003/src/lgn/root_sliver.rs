//! Root Sliver — {3}{G} — Creature — Sliver
//! Oracle: This spell can't be countered.
//! Oracle: Sliver spells can't be countered.
//! Set: LGN #137 — Legions | Scryfall ID: fdf5a106-5fb7-40e4-82a7-db559302a923 | Oracle ID: cb752313-dc7f-47fc-9077-9e3298e8f5fb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROOT_SLIVER,
    oracle_id = "cb752313-dc7f-47fc-9077-9e3298e8f5fb",
    scryfall_id = "fdf5a106-5fb7-40e4-82a7-db559302a923",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Root Sliver",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
