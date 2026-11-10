//! Hunter Sliver — {1}{R} — Creature — Sliver
//! Oracle: All Sliver creatures have provoke. (Whenever a Sliver attacks, its controller may have target creature defending player controls untap and block it if able.)
//! Set: LGN #102 — Legions | Scryfall ID: ca9aea1a-6f50-4f66-9f36-2e214dce41b4 | Oracle ID: f28b2ebe-49ef-4585-9bf2-7340abf3f0d3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HUNTER_SLIVER,
    oracle_id = "f28b2ebe-49ef-4585-9bf2-7340abf3f0d3",
    scryfall_id = "ca9aea1a-6f50-4f66-9f36-2e214dce41b4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Hunter Sliver",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
